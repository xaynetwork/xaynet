import threading
import time
from typing import Tuple

from absl import app, flags
import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.benchmark.net import load_lr_fn_fn, load_model_fn
from xain.datasets import load_splits
from xain.fl.participant import ModelProvider, Participant
from xain.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain.types import History, Metrics, Theta

FLAGS = flags.FLAGS

RETRY_TIMEOUT = 5
HEARTBEAT_TIME = 10


# TODO pull in heartbeat msg changes
def heartbeat(channel, terminate_event, selected_event):
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    while not terminate_event.is_set():
        # TODO include state with req? complicates things...
        reply = stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
        # TODO inspect reply... figure out whether to (un)set selected ev
        print(f"Participant received: {type(reply)}")
        time.sleep(HEARTBEAT_TIME)


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
    channel, theta_update: Tuple[Theta, int], history: History, metrics: Metrics
):
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    # build request starting with theta update
    theta, num = theta_update
    theta_update_p = coordinator_pb2.EndTrainingRequest.ThetaUpdate(
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
        theta_update=theta_update_p, history=h, metrics=m
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
    return Participant(cid, model_provider, xy_train, xy_val,
                       num_classes=10, batch_size=FLAGS.B)


def standby(channel, participant: Participant, hb: threading.Thread):
    # wait on hb until *selected* signal
    # ...
    # ready:
    theta, epochs, base = start_training(channel)
    # training:
    theta_n, his, _dict = participant.train_round(theta, epochs, base)
    met = participant.metrics()
    end_training(theta_n, his, met)
    # back to standby
    standby(channel, participant, hb)


def run():
    p = init_participant()

    # create a channel to the coordinator
    channel = grpc.insecure_channel("localhost:50051")

    # rendezvous with the coordinator
    rendezvous(channel)

    # start the heartbeat in a different thread
    terminate_event = threading.Event()
    heartbeat_thread = threading.Thread(
        target=heartbeat, args=(channel, terminate_event)
    )
    heartbeat_thread.start()

    # standby:
    standby(channel, p, heartbeat_thread)

    try:
        # never returns unless there is an exception
        heartbeat_thread.join()
    except KeyboardInterrupt:
        terminate_event.set()
        channel.close()


if __name__ == "__main__":
    flags.mark_flag_as_required("model")
    flags.mark_flag_as_required("dataset")
    flags.mark_flag_as_required("B")
    flags.mark_flag_as_required("partition_id")
    app.run(main=run)
