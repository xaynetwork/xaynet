from copy import deepcopy
import json
import enum
import time
import threading
from abc import ABC, abstractmethod
from typing import Any, Dict, List, Tuple, TypeVar, cast
import uuid

import numpy as np
from numpy import ndarray
from .http import AggregatorClient, CoordinatorClient

import logging


LOG=logging.getLogger("http")

class Participant(ABC):
    def __init__(self) -> None:
        super(Participant, self).__init__()

    @abstractmethod
    def init_weights(self) -> ndarray:
        pass

    @abstractmethod
    def train_round(
        self, weights: ndarray, epochs: int, epoch_base: int
    ) -> Tuple[ndarray, int]:
        pass


class DummyParticipant(Participant):
    def train_round(self, weights: ndarray, _epochs: int, _epoch_base: int):
        return weights

    def init_weights(self):
        np.ndarray([1, 2, 3, 4])


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
        self, coordinator_url: str, participant: Participant = DummyParticipant()
    ):
        self.state_record = StateRecord()
        self.participant = participant
        self.coordinator = CoordinatorClient(coordinator_url)
        self.aggregator = None

        self.exit_event = threading.Event()
        self.heartbeat_thread = None

    def run(self):
        self.rendez_vous()
        while True:
            with self.state_record:
                self.state_record.wait_until_selected_or_done()
                state, round = self.state_record.lookup()
                if state == State.DONE:
                    return
                elif state == State.TRAINING:
                    self.coordinator.start_training(self.id)
                    from IPython import embed; embed()

    def rendez_vous(self):
        self.id = self.coordinator.rendez_vous()["id"]
        self.start_heartbeat()

    def start_heartbeat(self):
        coordinator = deepcopy(self.coordinator)
        self.heartbeat_thread = HeartBeatWorker(
            coordinator, self.id, self.state_record, self.exit_event
        )
        self.heartbeat_thread.start()


class HeartBeatWorker(threading.Thread):
    def __init__(
        self,
        coordinator: CoordinatorClient,
        id: str,
        state_record: StateRecord,
        exit_event: threading.Event,
    ):
        self.coordinator = coordinator
        self.id = id
        self.state_record = state_record
        self.exit_event = exit_event
        super(HeartBeatWorker, self).__init__(name=f"heartbeat({self.id})", daemon=True)

    def run(self):
        LOG.debug("thread %s starting", self.name)
        try:
            while True:
                self.heartbeat()
                if self.exit_event.wait(timeout=5):
                    LOG.info("thread %s: exiting")
                    return
        except:
            LOG.exception("error while send heartbeat, exiting")
            self.exit_event.set()
            return

    def heartbeat(self):
        resp = self.coordinator.heartbeat(self.id)

        with self.state_record as state_record:
            current_state, current_round = state_record.lookup()
            state = resp["state"]

            # FIXME: The API should return proper JSON that would
            # make this much cleaner
            if state == "stand_by" and current_state != State.WAITING:
                state_record.set_state(State.STAND_BY)

            elif state == "finish" and current_state != State.DONE:
                state_record.set_state(State.DONE)

            elif state == "reject":
                state_record.set_state(State.DONE)

            elif state == "round":
                round = resp["round"]
                if current_state != State.TRAINING:
                    state_record.set_state(State.TRAINING)
                if current_round != round:
                    state_record.set_round(round)
