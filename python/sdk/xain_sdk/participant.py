from abc import ABC, abstractmethod
from copy import deepcopy
import enum
import logging
import sys
import threading
from typing import Optional, Tuple

from requests.exceptions import ConnectionError

from .http import (
    AggregatorClient,
    AnonymousCoordinatorClient,
    CoordinatorClient,
    StartTrainingRejected,
)
from .interfaces import TrainingInputABC, TrainingResultABC

LOG = logging.getLogger("http")


class ParticipantABC(ABC):
    @abstractmethod
    def init_weights(self) -> TrainingResultABC:
        raise NotImplementedError()

    @abstractmethod
    def train_round(self, training_input: TrainingInputABC) -> TrainingResultABC:
        raise NotImplementedError()

    @abstractmethod
    def deserialize_training_input(self, data: bytes) -> TrainingInputABC:
        raise NotImplementedError()


class State(enum.Enum):
    WAITING = 1
    TRAINING = 2
    DONE = 3


class StateRecord:
    def __init__(self, state: State = State.WAITING, round: int = -1) -> None:
        # By default, a re-entrant lock is used but we want a normal
        # lock here
        self.cond: threading.Condition = threading.Condition(threading.Lock())
        self.locked: bool = False
        self.round: int = round
        self.state: State = state
        self.dirty: bool = False

    def __enter__(self):
        self.cond.acquire()
        self.locked = True
        return self

    def __exit__(self, *args, **kwargs):
        if self.dirty:
            self.cond.notify()
            self.dirty = False
        self.locked = False
        self.cond.release()

    def assert_locked(self):
        if not self.locked:
            raise RuntimeError("StateRecord must be locked")

    def lookup(self) -> Tuple[State, int]:
        self.assert_locked()
        return self.state, self.round

    def set_state(self, state: State) -> None:
        self.assert_locked()
        self.state = state
        self.dirty = True

    def set_round(self, round: int) -> None:
        self.assert_locked()
        self.round = round
        self.dirty = True

    def wait_until_selected_or_done(self) -> State:
        self.assert_locked()
        # wait() releases the lock. It's fine to set the `locked`
        # attribute, because until wait() runs, the lock won't be
        # released so no other thread will try to access this attribute.
        #
        # It's also fine to re-set the attribute to True afterward,
        # because this thread will hold the lock at this point.
        #
        # FIXME: explain better why this it is safe
        self.locked = False
        self.cond.wait_for(lambda: self.state in {State.TRAINING, State.DONE})
        self.locked = True
        return self.state


class InternalParticipant:
    def __init__(
        self,
        participant: ParticipantABC,
        coordinator_url: str,
        heartbeat_frequency: float,
    ):
        self.state_record = StateRecord()
        self.participant = participant
        self.heartbeat_frequency = heartbeat_frequency

        self.anonymous_client = AnonymousCoordinatorClient(coordinator_url)
        self.coordinator_client: Optional[CoordinatorClient] = None
        self.aggregator_client: Optional[AggregatorClient] = None

        self.exit_event = threading.Event()
        self.heartbeat_thread = None

    def run(self) -> None:
        try:
            self._run()
        except InterruptedError:
            LOG.warning("exiting: interrupt signal caught")
            self.exit_event.set()
            sys.exit(0)

    def _run(self) -> None:
        self.rendez_vous()
        while not self.exit_event.is_set():
            LOG.info("waiting for being selected")
            with self.state_record:
                self.state_record.wait_until_selected_or_done()
                new_state, _ = self.state_record.lookup()

            if new_state == State.DONE:
                LOG.info("state changed: DONE")
                self.exit_event.set()
                return

            if new_state == State.TRAINING:
                LOG.info("state changed: TRAINING")
                self.train()
                continue

            raise ParticipantError(f"unexpected state: {new_state}")

    def train(self) -> None:
        try:
            LOG.info("requesting training information to the coordinator")
            assert self.coordinator_client is not None
            self.aggregator_client = self.coordinator_client.start_training()
        except StartTrainingRejected:
            LOG.warning("start training request rejected")
            with self.state_record:
                self.state_record.set_state(State.WAITING)

        LOG.info("downloading global weights from the aggregator")
        assert self.aggregator_client is not None
        data = self.aggregator_client.download()
        LOG.info("retrieved training data (length: %d bytes)", len(data))
        training_input = self.participant.deserialize_training_input(data)

        if training_input.is_initialization_round():
            LOG.info("initializing the weights")
            result = self.participant.init_weights()
        else:
            LOG.info("training")
            result = self.participant.train_round(training_input)
            assert isinstance(result, TrainingResultABC)
            LOG.info("training finished")

        LOG.info("sending the local weights to the aggregator")
        assert self.aggregator_client is not None
        self.aggregator_client.upload(result.tobytes())

        LOG.info("going back to WAITING state")
        with self.state_record:
            self.state_record.set_state(State.WAITING)

    def rendez_vous(self):
        try:
            self.coordinator_client = self.anonymous_client.rendez_vous()
        except ConnectionError as err:
            LOG.error("rendez vous failed: %s", err)
            raise ParticipantError("Rendez-vous request failed")
        self.start_heartbeat()

    def start_heartbeat(self):
        self.heartbeat_thread = HeartBeatWorker(
            deepcopy(self.coordinator_client),
            self.state_record,
            self.exit_event,
            self.heartbeat_frequency,
        )
        self.heartbeat_thread.start()


class HeartBeatWorker(threading.Thread):
    def __init__(
        self,
        coordinator_client: CoordinatorClient,
        state_record: StateRecord,
        exit_event: threading.Event,
        heartbeat_frequency: float,
    ):
        self.coordinator_client = coordinator_client
        self.state_record = state_record
        self.exit_event = exit_event
        self.heartbeat_frequency = heartbeat_frequency
        super(HeartBeatWorker, self).__init__(daemon=True)

    def run(self):
        LOG.debug("heartbeat thread starting")
        try:
            while True:
                self.heartbeat()
                if self.exit_event.wait(timeout=self.heartbeat_frequency):
                    LOG.debug("heartbeat worker exiting: exit flag set in main thead")
                    return
        except Exception:  # pylint: disable=broad-except
            LOG.exception("error while sending heartbeat, exiting")
            with self.state_record as state_record:
                state_record.set_state(State.DONE)
            return

    def heartbeat(self):
        resp = self.coordinator_client.heartbeat()

        with self.state_record as state_record:
            current_state, current_round = state_record.lookup()
            state = resp["state"]

            # FIXME: The API should return proper JSON that would
            # make this much cleaner
            if state == "stand_by" and current_state != State.WAITING:
                state_record.set_state(State.WAITING)

            elif state == "finish" and current_state != State.DONE:
                state_record.set_state(State.DONE)

            elif state == "reject":
                LOG.error("hearbeat rejected")
                state_record.set_state(State.DONE)

            elif state == "round":
                round = resp["round"]
                if current_state != State.TRAINING:
                    state_record.set_state(State.TRAINING)
                if current_round != round:
                    state_record.set_round(round)


class ParticipantError(Exception):
    pass
