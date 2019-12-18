"""Module implementing the networked Participant using gRPC."""

import threading
import time
from enum import Enum, auto
from typing import Dict, List, Tuple

from grpc import Channel, insecure_channel
from numproto import ndarray_to_proto, proto_to_ndarray
from numpy import ndarray

from xain_fl.cproto.coordinator_pb2 import (
    EndTrainingReply,
    EndTrainingRequest,
    HeartbeatReply,
    HeartbeatRequest,
    RendezvousReply,
    RendezvousRequest,
    RendezvousResponse,
    StartTrainingReply,
    StartTrainingRequest,
    State,
)
from xain_fl.cproto.coordinator_pb2_grpc import CoordinatorStub
from xain_fl.logger import get_logger
from xain_fl.sdk.participant import Participant

logger = get_logger(__name__)


# timings in seconds
RETRY_TIMEOUT: int = 5
HEARTBEAT_TIME: int = 10


class ParState(Enum):
    """Enumeration of Participant states."""

    WAITING_FOR_SELECTION: auto = auto()
    TRAINING: auto = auto()
    POST_TRAINING: auto = auto()
    DONE: auto = auto()


def rendezvous(channel: Channel) -> None:
    """Start a rendezvous exchange with a coordinator.

    Args:
        channel (~grpc.Channel): A gRPC channel to the coordinator.
    """

    coordinator: CoordinatorStub = CoordinatorStub(channel=channel)

    response: RendezvousResponse = RendezvousResponse.LATER
    reply: RendezvousReply
    while response == RendezvousResponse.LATER:
        reply = coordinator.Rendezvous(request=RendezvousRequest())
        if reply.response == RendezvousResponse.ACCEPT:
            logger.info("Participant received: ACCEPT")
        elif reply.response == RendezvousResponse.LATER:
            logger.info(
                "Participant received: LATER. Retrying...", retry_timeout=RETRY_TIMEOUT
            )
            time.sleep(RETRY_TIMEOUT)

        response = reply.response


def start_training(channel: Channel) -> Tuple[List[ndarray], int, int]:
    """Start a training initiation exchange with a coordinator.

    The decoded contents of the response from the coordinator are returned.

    Args:
        channel (~grpc.Channel): A gRPC channel to the coordinator.

    Returns:
        ~typing.List[~numpy.ndarray]: The weights of a global model to train on.
        int: The number of epochs to train.
        int: The epoch base of the global model.
    """

    coordinator: CoordinatorStub = CoordinatorStub(channel=channel)

    # send request to start training
    reply: StartTrainingReply = coordinator.StartTraining(
        request=StartTrainingRequest()
    )
    logger.info("Participant received", reply_type=type(reply))

    weights: List[ndarray] = [proto_to_ndarray(weight) for weight in reply.weights]
    epochs: int = reply.epochs
    epoch_base: int = reply.epoch_base

    return weights, epochs, epoch_base


def end_training(
    channel: Channel,
    weights: List[ndarray],
    number_samples: int,
    metrics: Dict[str, List[ndarray]],
) -> None:
    """Start a training completion exchange with a coordinator.

    The locally trained weights and the number of samples as well as metrics metadata is sent.

    Args:
        channel (~grpc.Channel): A gRPC channel to the coordinator.
        weights (~typing.List[~numpy.ndarray]): The weights of the locally trained model.
        number_samples (int): The number of samples in the training dataset.
        metrics (~typing.Dict[str, ~numpy.ndarray]): Metrics metadata.
    """

    coordinator: CoordinatorStub = CoordinatorStub(channel=channel)

    # build request starting with weight update
    weights_proto: List = [ndarray_to_proto(weight) for weight in weights]

    # metric data containing the metric names mapped to Metrics as protobuf message
    metrics_proto: Dict[str, EndTrainingRequest.Metrics] = {
        key: EndTrainingRequest.Metrics(
            metrics=[ndarray_to_proto(value) for value in values]
        )
        for key, values in metrics.items()
    }

    # assembling a request with the update of the weights and the metrics
    request: EndTrainingRequest = EndTrainingRequest(
        weights=weights_proto, number_samples=number_samples, metrics=metrics_proto
    )
    reply: EndTrainingReply = coordinator.EndTraining(request=request)
    logger.info("Participant received", reply_type=type(reply))


def training_round(channel: Channel, participant: Participant) -> None:
    """Initiate a training round exchange with a coordinator.

    Begins with `start_training`. Then performs local training computation using the `participant`.
    Finally, completes with `end_training`.

    Args:
        channel (~grpc.Channel): A gRPC channel to the coordinator.
        participant (~xain_sdk.participant.Participant): The local participant.
    """

    # retreive global weights, epochs and epoch base from the coordinator
    weights: List[ndarray]
    epochs: int
    epoch_base: int
    weights, epochs, epoch_base = start_training(channel=channel)

    # start a local training round of the participant
    number_samples: int
    metrics: Dict[str, List[ndarray]]
    weights, number_samples, metrics = participant.train_round(
        weights=weights, epochs=epochs, epoch_base=epoch_base
    )

    # return updated weights, number of training samples and metrics metadata to the coordinator
    end_training(
        channel=channel, weights=weights, number_samples=number_samples, metrics=metrics
    )


class StateRecord:
    """Thread-safe record of a participant's state and round number."""

    def __init__(  # pylint: disable=redefined-builtin
        self, state: ParState = ParState.WAITING_FOR_SELECTION, round: int = 0
    ) -> None:
        """Initialize the state record.

        Args:
            state (~xain_sdk.participant_state_machine.ParState): The initial state. Defaults to
                WAITING_FOR_SELECTION.
            round (int): The initial training round. Defaults to 0.
        """

        self.cond: threading.Condition = threading.Condition()
        self.round: int = round
        self.state: ParState = state

    def lookup(self) -> Tuple[ParState, int]:
        """Get the state and round number.

        Returns:
            ~typing.Tuple[~xain_sdk.participant_state_machine.ParState, int]: The state and round
                number.
        """

        with self.cond:
            return self.state, self.round

    def update(self, state: ParState) -> None:
        """Update the state.

        Args:
            state (~xain_sdk.participant_state_machine.ParState): The state to update to.
        """

        with self.cond:
            self.state = state
            self.cond.notify()

    def wait_until_selected_or_done(self) -> ParState:
        """Wait until the participant was selected for training or is done.

        Returns:
            ~xain_sdk.participant_state_machine.ParState: The new state the participant is in.
        """

        with self.cond:
            self.cond.wait_for(lambda: self.state in {ParState.TRAINING, ParState.DONE})
            return self.state

    def wait_until_next_round(self) -> ParState:
        """Wait until the participant can start into the next round of training.

        Returns:
            ~xain_sdk.participant_state_machine.ParState: The new state the participant is in.
        """

        with self.cond:
            self.cond.wait_for(
                lambda: self.state
                in {ParState.TRAINING, ParState.WAITING_FOR_SELECTION, ParState.DONE}
            )
            return self.state


def transit(state_record: StateRecord, heartbeat_reply: HeartbeatReply) -> None:
    """Participant state transition function on a heartbeat response. Updates the state record.

    Args:
        state_record (~xain_sdk.participant_state_machine.StateRecord): The updatable state record
            of the participant.
        heartbeat_reply (~xain_sdk.cproto.coordinator_pb2.HeartbeatReply): The heartbeat reply from
            the coordinator.
    """

    state: State = heartbeat_reply.state
    round: int = heartbeat_reply.round  # pylint: disable=redefined-builtin
    with state_record.cond:
        if state_record.state == ParState.WAITING_FOR_SELECTION:
            if state == State.ROUND:
                state_record.state = ParState.TRAINING
                state_record.round = round
                state_record.cond.notify()
            elif state == State.FINISHED:
                state_record.state = ParState.DONE
                state_record.cond.notify()
        elif state_record.state == ParState.POST_TRAINING:
            if state == State.STANDBY:
                # not selected
                state_record.state = ParState.WAITING_FOR_SELECTION
                # prob ok to keep state_record.round as it is
                state_record.cond.notify()
            elif state == State.ROUND and round == state_record.round + 1:
                state_record.state = ParState.TRAINING
                state_record.round = round
                state_record.cond.notify()
            elif state == State.FINISHED:
                state_record.state = ParState.DONE
                state_record.cond.notify()


def message_loop(
    channel: Channel, state_record: StateRecord, terminate: threading.Event
) -> None:
    """Periodically send (and handle) heartbeat messages in a loop.

    Args:
        channel (~grpc.Channel): A gRPC channel to the coordinator.
        state_record (~xain_sdk.participant_state_machine.StateRecord): The participant's state
            record.
        terminate (~threading.Event): An event to terminate the message loop.
    """

    coordinator: CoordinatorStub = CoordinatorStub(channel=channel)
    while not terminate.is_set():
        transit(
            state_record=state_record,
            heartbeat_reply=coordinator.Heartbeat(request=HeartbeatRequest()),
        )
        time.sleep(HEARTBEAT_TIME)


def start_participant(participant: Participant, coordinator_url: str) -> None:
    """Top-level function for the participant's state machine.

    After rendezvous and heartbeat initiation, the Participant is WAITING_FOR_SELECTION. When
    selected, it moves to TRAINING followed by POST_TRAINING. If selected again for the next round,
    it moves back to TRAINING, otherwise it is back to WAITING_FOR_SELECTION.

    Args:
        participant (~xain_sdk.participant.Participant): The participant for local training.
        coordinator_url (str): The URL of the coordinator to connect to.
    """

    # use insecure channel for now
    with insecure_channel(target=coordinator_url) as channel:  # thread-safe
        rendezvous(channel=channel)

        state_record: StateRecord = StateRecord()
        terminate: threading.Event = threading.Event()
        msg_loop = threading.Thread(
            target=message_loop, args=(channel, state_record, terminate)
        )
        msg_loop.start()

        # in WAITING_FOR_SELECTION state
        begin_selection_wait(
            state_record=state_record, channel=channel, participant=participant
        )

        # possibly several training rounds later... in DONE state
        terminate.set()
        msg_loop.join()


def begin_selection_wait(
    state_record: StateRecord, channel: Channel, participant: Participant
) -> None:
    """Perform actions in Participant state WAITING_FOR_SELECTION.

    Args:
        state_record (~xain_sdk.participant_state_machine.StateRecord): The participant's state
            record.
        channel (~grpc.Channel): A gRPC channel to the coordinator.
        participant (~xain_sdk.participant.Participant): The participant for local training.
    """

    state: ParState = state_record.wait_until_selected_or_done()
    if state == ParState.TRAINING:
        # selected
        begin_training(
            state_record=state_record, channel=channel, participant=participant
        )
    elif state == ParState.DONE:
        pass


def begin_training(
    state_record: StateRecord, channel: Channel, participant: Participant
) -> None:
    """Perform actions in Participant state TRAINING and POST_TRAINING.

    Args:
        state_record (~xain_sdk.participant_state_machine.StateRecord): The participant's state
            record.
        channel (~grpc.Channel): A gRPC channel to the coordinator.
        participant (~xain_sdk.participant.Participant): The participant for local training.
    """

    # perform the training procedures
    training_round(channel=channel, participant=participant)

    # move to POST_TRAINING state
    state_record.update(state=ParState.POST_TRAINING)
    state: ParState = state_record.wait_until_next_round()
    if state == ParState.TRAINING:
        # selected again
        begin_training(
            state_record=state_record, channel=channel, participant=participant
        )
    elif state == ParState.WAITING_FOR_SELECTION:
        # not this time
        begin_selection_wait(
            state_record=state_record, channel=channel, participant=participant
        )
    elif state == ParState.DONE:
        # that was the last round
        pass
