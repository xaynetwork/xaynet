import threading
import time
from enum import Enum, auto
from typing import Tuple

import grpc
from absl import app, flags
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.benchmark.net import load_lr_fn_fn, load_model_fn, model_fns
from xain.datasets import load_splits
from xain.fl.participant import ModelProvider, Participant
from xain.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain.types import History, Metrics, Theta

FLAGS = flags.FLAGS

flags.DEFINE_string("model", None, f"Model name, one of {[fn for fn in model_fns]}")
flags.DEFINE_string("dataset", None, "Dataset name")
flags.DEFINE_integer("B", None, "Batch size")
flags.DEFINE_integer("partition_id", None, "Partition ID for unitary training")

RETRY_TIMEOUT = 5
HEARTBEAT_TIME = 10


class ParState(Enum):
    WAITING_FOR_SELECTION = auto()
    TRAINING = auto()
    POST_TRAINING = auto()
    DONE = auto()


# def heartbeat(channel, terminate_event, selected_event):
#     stub = coordinator_pb2_grpc.CoordinatorStub(channel)
#     while not terminate_event.is_set():
#         # exchange a heartbeat
#         reply = stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
#         print(f"Participant received: {type(reply)}")
#         if reply.state == coordinator_pb2.State.FINISHED:
#             terminate_event.set()
#             return
#         if reply.state == coordinator_pb2.State.ROUND:
#             # signal "round open" to main thread
#             selected_event.set()
#         # not much to do for State.STANDBY (still waiting registrations)
#         time.sleep(HEARTBEAT_TIME)


def rendezvous(channel):
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    response = coordinator_pb2.RendezvousResponse.LATER

    while response == coordinator_pb2.RendezvousResponse.LATER:
        reply = stub.Rendezvous(coordinator_pb2.RendezvousRequest())
        if reply.response == coordinator_pb2.RendezvousResponse.ACCEPT:
            print("Participant received: ACCEPT")
        elif reply.response == coordinator_pb2.RendezvousResponse.LATER:
            print(f"Participant received: LATER. Retrying in {RETRY_TIMEOUT}s")
            time.sleep(RETRY_TIMEOUT)

        response = reply.response


def start_training(channel) -> Tuple[Theta, int, int]:
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


def init_participant() -> Participant:
    xy_train_partitions, xy_val, _xy_test = load_splits(FLAGS.dataset)

    model_fn = load_model_fn(FLAGS.model)
    lr_fn_fn = load_lr_fn_fn(FLAGS.model)
    model_provider = ModelProvider(model_fn, lr_fn_fn)

    cid = 0
    xy_train = xy_train_partitions[FLAGS.partition_id]
    return Participant(
        cid, model_provider, xy_train, xy_val, num_classes=10, batch_size=FLAGS.B
    )


def training_round(channel, participant: Participant):
    theta, epochs, base = start_training(channel)
    # training:
    theta_n, his, _dict = participant.train_round(theta, epochs, base)
    # NOTE _dict is the opt_config - ignore for now
    met = participant.metrics()
    end_training(channel, theta_n, his, met)


# def wait_selected(selected: threading.Event):
#     # wait on heartbeat until *selected* event signals
#     while not selected.wait(RETRY_TIMEOUT):
#         print(f"Not yet selected for round. Retrying in {RETRY_TIMEOUT}s")
#     selected.clear()


# def run(part: Participant):
#     # create (for now, insecure) channel to coordinator
#     with grpc.insecure_channel("localhost:50051") as channel:
#         rendezvous(channel)
#         print("rendezvoused")

#         # start heartbeat in different thread
#         terminate_event = threading.Event()
#         selected_event = threading.Event()
#         heartbeat_thread = threading.Thread(
#             target=heartbeat, args=(channel, terminate_event, selected_event)
#         )
#         heartbeat_thread.start()
#         print("heartbeat started")

#         # if get aborted in this thread at least signal heartbeat thread to finish
#         try:
#             while not terminate_event.is_set():
#                 # standby:
#                 wait_selected(selected_event)
#                 # ready:
#                 training_round(channel, part)
#         except KeyboardInterrupt:
#             terminate_event.set()


def main(_argv):
    print(f"model: {FLAGS.model}")
    print(f"dataset: {FLAGS.dataset}")
    print(f"B: {FLAGS.B}")
    print(f"partition_id: {FLAGS.partition_id}")
    go(init_participant())


if __name__ == "__main__":
    flags.mark_flag_as_required("model")
    flags.mark_flag_as_required("dataset")
    flags.mark_flag_as_required("B")
    flags.mark_flag_as_required("partition_id")
    app.run(main)


class StateRecord:
    def __init__(self):
        self.cv = threading.Condition()
        self.round = 0
        self.state = ParState.WAITING_FOR_SELECTION

    def lookup(self):
        with self.cv:
            return self.state, self.round

    # possibly useful for TRAINING -> POST_TRAINING
    def update(self, state):
        with self.cv:
            self.state = state
            self.cv.notify()

    def wait_until_selected_or_done(self):
        with self.cv:
            self.cv.wait_for(lambda: self.state in {ParState.TRAINING, ParState.DONE})
            # which one was it?
            return self.state

    def wait_until_next_round(self):
        with self.cv:
            self.cv.wait_for(
                lambda: self.state
                in {ParState.TRAINING, ParState.WAITING_FOR_SELECTION}
            )
            # which one was it?
            return self.state


# updates st
def transit(st, beat_reply):
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
    coord = coordinator_pb2_grpc.CoordinatorStub(chan)
    while not terminate.is_set():
        req = coordinator_pb2.HeartbeatRequest()
        reply = coord.Heartbeat(req)
        transit(st, reply)
        time.sleep(HEARTBEAT_TIME)


def go(part):
    with grpc.insecure_channel("localhost:50051") as chan:
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
    ps = st.wait_until_selected_or_done()
    if ps == ParState.TRAINING:
        # selected
        begin_training(st, chan, part)
    elif ps == ParState.DONE:
        pass


def begin_training(st, chan, part):
    # perform the training procedures
    training_round(chan, part)
    # move to POST_TRAINING state
    st.update(ParState.POST_TRAINING)
    ps = st.wait_until_next_round()
    if ps == ParState.TRAINING:
        begin_training(st, chan, part)
    elif ps == ParState.WAITING_FOR_SELECTION:
        begin_selection_wait(st, chan, part)
    elif ps == ParState.DONE:
        pass
