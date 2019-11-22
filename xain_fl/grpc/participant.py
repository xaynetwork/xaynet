"""Module implementing the networked Participant using gRPC.
"""
import threading
import time
from enum import Enum, auto
from typing import Tuple

import grpc
from absl import app, flags
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.datasets import load_splits
from xain_fl.fl.participant import ModelProvider, Participant
from xain_fl.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain_fl.types import History, Metrics, Theta

FLAGS = flags.FLAGS

# flags.DEFINE_string(
#     "model_name", None, f"Model name, one of {[fn for fn in model_fns]}"
# )
flags.DEFINE_string("dataset_name", None, "Dataset name")
flags.DEFINE_integer("batch_size", None, "Batch size")
flags.DEFINE_integer("partition_iden", None, "Partition ID for unitary training")

RETRY_TIMEOUT = 5
HEARTBEAT_TIME = 10


class ParState(Enum):
    """Enumeration of Participant states.
    """

    WAITING_FOR_SELECTION = auto()
    TRAINING = auto()
    POST_TRAINING = auto()
    DONE = auto()


class StateRecord:
    """Thread-safe record of Participant state and round number.
    """

    def __init__(self):
        self.cv = threading.Condition()
        self.round = 0
        self.state = ParState.WAITING_FOR_SELECTION

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
            if msg == coordinator_pb2.State.Value("ROUND"):
                st.state = ParState.TRAINING
                st.round = r
                st.cv.notify()
            elif msg == coordinator_pb2.State.Value("FINISHED"):
                st.state = ParState.DONE
                st.cv.notify()
        elif st.state == ParState.POST_TRAINING:
            if msg == coordinator_pb2.State.Value("STANDBY"):
                # not selected
                st.state = ParState.WAITING_FOR_SELECTION
                # prob ok to keep st.round as it is
                st.cv.notify()
            elif msg == coordinator_pb2.State.Value("ROUND") and r == st.round + 1:
                st.state = ParState.TRAINING
                st.round = r
                st.cv.notify()
            elif msg == coordinator_pb2.State.Value("FINISHED"):
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
        if st.state in [ParState.WAITING_FOR_SELECTION, ParState.POST_TRAINING, ParState.DONE]:
            state = coordinator_pb2.State.Value("READY")
        else:
            state = coordinator_pb2.State.Value("TRAINING")
        req = coordinator_pb2.HeartbeatRequest(state=state, round=st.round)
        reply = coord.Heartbeat(req)
        transit(st, reply)
        time.sleep(HEARTBEAT_TIME)


def go(part: Participant, coordinator_address: str):
    """Top-level function for the Participant state machine.

    After rendezvous and heartbeat initiation, the Participant is
    WAITING_FOR_SELECTION. When selected, it moves to TRAINING followed by
    POST_TRAINING. If selected again for the next round, it moves back to
    TRAINING, otherwise it is back to WAITING_FOR_SELECTION.

    Args:
        part (obj:`Participant`): Participant object for training computation.
    """

    # channel options
    options = [
        ("grpc.max_receive_message_length", -1),
        ("grpc.max_send_message_length", -1),
    ]
    # use insecure channel for now
    with grpc.insecure_channel(
        target=coordinator_address, options=options
    ) as chan:  # thread-safe
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


# deprecated: see message_loop
def heartbeat(channel, terminate_event):
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    while not terminate_event.is_set():
        reply = stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
        print(f"Participant received: {type(reply)}")
        time.sleep(HEARTBEAT_TIME)


def rendezvous(channel):
    """Starts a rendezvous exchange with Coordinator.

    Args:
        channel: gRPC channel to Coordinator.
    """
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    response = coordinator_pb2.RendezvousResponse.Value("LATER")

    while response == coordinator_pb2.RendezvousResponse.Value("LATER"):
        reply = stub.Rendezvous(coordinator_pb2.RendezvousRequest())
        if reply.response == coordinator_pb2.RendezvousResponse.Value("ACCEPT"):
            print("Participant received: ACCEPT")
        elif reply.response == coordinator_pb2.RendezvousResponse.Value("LATER"):
            print(f"Participant received: LATER. Retrying in {RETRY_TIMEOUT}s")
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
    print(f"Participant received: {type(reply)}")
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
    print(f"Participant received: {type(reply)}")


# def init_participant() -> Participant:
#     """Initialises a local Participant configured with command line flags.
# 
#     Returns:
#         obj:`Participant`: Participant object.
#     """
#     xy_train_partitions, xy_val, _xy_test = load_splits(FLAGS.dataset_name)
# 
#     model_fn = load_model_fn(FLAGS.model_name)
#     lr_fn_fn = load_lr_fn_fn(FLAGS.model_name)
#     model_provider = ModelProvider(model_fn, lr_fn_fn)
# 
#     cid = 0
#     xy_train = xy_train_partitions[FLAGS.partition_iden]
#     return Participant(
#         cid,
#         model_provider,
#         xy_train,
#         xy_val,
#         num_classes=10,
#         batch_size=FLAGS.batch_size,
#     )


def training_round(channel, participant: Participant):
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


def main(_argv):
    print(f"model_name: {FLAGS.model_name}")
    print(f"dataset_name: {FLAGS.dataset_name}")
    print(f"batch_size: {FLAGS.batch_size}")
    print(f"partition_iden: {FLAGS.partition_iden}")
    # go(init_participant())


if __name__ == "__main__":
    flags.mark_flag_as_required("model_name")
    flags.mark_flag_as_required("dataset_name")
    flags.mark_flag_as_required("batch_size")
    flags.mark_flag_as_required("partition_iden")
    app.run(main)
