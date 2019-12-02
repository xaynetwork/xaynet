"""Module implementing the networked Participant using gRPC.
"""
import os
import threading
import time
from enum import Enum, auto
from typing import Tuple

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain_fl.logger import get_logger
from xain_fl.types import History, Metrics, Theta

RETRY_TIMEOUT = 5
HEARTBEAT_TIME = 10

logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


class ParState(Enum):
    """Enumeration of Participant states.
    """

    WAITING_FOR_SELECTION = auto()
    TRAINING = auto()
    POST_TRAINING = auto()
    DONE = auto()


def rendezvous(channel):
    """Starts a rendezvous exchange with Coordinator.

    Args:
        channel: gRPC channel to Coordinator.
    """
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    response = coordinator_pb2.RendezvousResponse.LATER

    while response == coordinator_pb2.RendezvousResponse.LATER:
        reply = stub.Rendezvous(coordinator_pb2.RendezvousRequest())
        if reply.response == coordinator_pb2.RendezvousResponse.ACCEPT:
            logger.info("Participant received: ACCEPT")
        elif reply.response == coordinator_pb2.RendezvousResponse.LATER:
            logger.info("Participant received: LATER. Retrying in %s", RETRY_TIMEOUT)
            time.sleep(RETRY_TIMEOUT)

        response = reply.response


def start_training(channel) -> Tuple[Theta, int, int]:
    """Starts a training initiation exchange with Coordinator. Returns the decoded
    contents of the response from Coordinator.

    Args:
        channel: gRPC channel to Coordinator.

    Returns:
        obj:`Theta`: Global model to train on.
        obj:`int`: Number of epochs.
        obj:`int`: Epoch base.
    """
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    req = coordinator_pb2.StartTrainingRequest()
    # send request to start training
    reply = stub.StartTraining(req)
    logger.info("Participant received: %s", type(reply))
    theta, epochs, epoch_base = reply.theta, reply.epochs, reply.epoch_base
    return [proto_to_ndarray(pnda) for pnda in theta], epochs, epoch_base


def end_training(
    channel, theta_n: Tuple[Theta, int], history: History, metrics: Metrics
):
    """Starts a training completion exchange with Coordinator, sending a locally
    trained model and metadata.

    Args:
        channel: gRPC channel to Coordinator.
        theta_n (obj:`Tuple[Theta, int]`): Locally trained model.
        history (obj:`History`): History metadata.
        Metrics (obj:`Metrics`): Metrics metadata.
    """
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    # build request starting with theta update
    theta, num = theta_n
    theta_n_proto = coordinator_pb2.EndTrainingRequest.ThetaUpdate(
        theta_prime=[ndarray_to_proto(nda) for nda in theta], num_examples=num
    )
    # history data
    h = {
        k: coordinator_pb2.EndTrainingRequest.HistoryValue(values=v)
        for k, v in history.items()
    }
    # metrics
    cid, vbc = metrics
    m = coordinator_pb2.EndTrainingRequest.Metrics(cid=cid, vol_by_class=vbc)
    # assemble req
    req = coordinator_pb2.EndTrainingRequest(
        theta_update=theta_n_proto, history=h, metrics=m
    )
    # send request to end training
    reply = stub.EndTraining(req)
    logger.info("Participant received: %s", type(reply))


def training_round(channel, participant):
    """Initiates training round exchange with Coordinator.

    Begins with `start_training`. Then performs local training computation using
    `participant`. Finally, completes with `end_training`.

    Args:
        channel: gRPC channel to Coordinator.
        participant (obj:`Participant`): Local Participant.
    """
    theta, epochs, base = start_training(channel)
    # training:
    theta_n, his, _dict = participant.train_round(theta, epochs, base)
    # NOTE _dict is the opt_config - ignore for now
    met = participant.metrics()
    end_training(channel, theta_n, his, met)


class StateRecord:
    """Thread-safe record of Participant state and round number.
    """

    # pylint: disable=W0622
    def __init__(self, state=ParState.WAITING_FOR_SELECTION, round=0):
        self.cv = threading.Condition()
        self.round = round
        self.state = state

    def lookup(self):
        """Looks up the state and round number.

        Returns:
            :obj:`Tuple[ParState, int]`: State and round number
        """
        with self.cv:
            return self.state, self.round

    def update(self, state):
        """Updates state.

        Args:
            state (:obj:`ParState`): State to update to.
        """
        with self.cv:
            self.state = state
            self.cv.notify()

    def wait_until_selected_or_done(self):
        """Waits until Participant is in the state of having been selected for training
        (or is completely done).

        Returns:
            :obj:`ParState`: New state Participant is in.
        """
        with self.cv:
            self.cv.wait_for(lambda: self.state in {ParState.TRAINING, ParState.DONE})
            # which one was it?
            return self.state

    def wait_until_next_round(self):
        """Waits until Participant is in a state indicating the start of the next round
        of training.

        Returns:
            :obj:`ParState`: New state Participant is in.
        """
        with self.cv:
            self.cv.wait_for(
                lambda: self.state
                in {ParState.TRAINING, ParState.WAITING_FOR_SELECTION, ParState.DONE}
            )
            # which one was it?
            return self.state


def transit(st, beat_reply):
    """Participant state transition function on a heartbeat response. Updates the
    state record `st`.

    Args:
        st (obj:`StateRecord`): Participant state record to update.
        beat_reply (obj:`coordinator_pb2.HeartbeatReply`): Heartbeat from Coordinator.
    """
    msg, r = beat_reply.state, beat_reply.round
    with st.cv:
        if st.state == ParState.WAITING_FOR_SELECTION:
            if msg == coordinator_pb2.State.ROUND:
                st.state = ParState.TRAINING
                st.round = r
                st.cv.notify()
            elif msg == coordinator_pb2.State.FINISHED:
                st.state = ParState.DONE
                st.cv.notify()
        elif st.state == ParState.POST_TRAINING:
            if msg == coordinator_pb2.State.STANDBY:
                # not selected
                st.state = ParState.WAITING_FOR_SELECTION
                # prob ok to keep st.round as it is
                st.cv.notify()
            elif msg == coordinator_pb2.State.ROUND and r == st.round + 1:
                st.state = ParState.TRAINING
                st.round = r
                st.cv.notify()
            elif msg == coordinator_pb2.State.FINISHED:
                st.state = ParState.DONE
                st.cv.notify()


def message_loop(chan, st, terminate):
    """Periodically sends (and handles) heartbeat messages in a loop.

    Args:
        chan: gRPC channel to Coordinator.
        st (obj:`StateRecord`): Participant state record.
        terminate (obj:`threading.Event`): Event to terminate message loop.
    """
    coord = coordinator_pb2_grpc.CoordinatorStub(chan)
    while not terminate.is_set():
        req = coordinator_pb2.HeartbeatRequest()
        reply = coord.Heartbeat(req)
        transit(st, reply)
        time.sleep(HEARTBEAT_TIME)


def go(part):
    """Top-level function for the Participant state machine.

    After rendezvous and heartbeat initiation, the Participant is
    WAITING_FOR_SELECTION. When selected, it moves to TRAINING followed by
    POST_TRAINING. If selected again for the next round, it moves back to
    TRAINING, otherwise it is back to WAITING_FOR_SELECTION.

    Args:
        part (obj:`Participant`): Participant object for training computation.
    """
    # use insecure channel for now
    with grpc.insecure_channel("localhost:50051") as chan:  # thread-safe
        rendezvous(chan)

        st = StateRecord()
        terminate = threading.Event()
        ml = threading.Thread(target=message_loop, args=(chan, st, terminate))
        ml.start()

        # in WAITING_FOR_SELECTION state
        begin_selection_wait(st, chan, part)

        # possibly several training rounds later...
        # in DONE state
        terminate.set()
        ml.join()


def begin_selection_wait(st, chan, part):
    """Perform actions in Participant state WAITING_FOR_SELECTION.

    Args:
        st (obj:`StateRecord`): Participant state record.
        chan: gRPC channel to Coordinator.
        part (obj:`Participant`): Participant object for training computation.
    """
    ps = st.wait_until_selected_or_done()
    if ps == ParState.TRAINING:
        # selected
        begin_training(st, chan, part)
    elif ps == ParState.DONE:
        pass


def begin_training(st, chan, part):
    """Perform actions in Participant state TRAINING and POST_TRAINING.

    Args:
        st (obj:`StateRecord`): Participant state record.
        chan: gRPC channel to Coordinator.
        part (obj:`Participant`): Participant object for training computation.
    """
    # perform the training procedures
    training_round(chan, part)
    # move to POST_TRAINING state
    st.update(ParState.POST_TRAINING)
    ps = st.wait_until_next_round()
    if ps == ParState.TRAINING:
        # selected again
        begin_training(st, chan, part)
    elif ps == ParState.WAITING_FOR_SELECTION:
        # not this time
        begin_selection_wait(st, chan, part)
    elif ps == ParState.DONE:
        # that was the last round
        pass
