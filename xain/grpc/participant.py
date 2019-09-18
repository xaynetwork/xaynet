import threading
import time

import grpc

from xain.grpc import coordinator_pb2, coordinator_pb2_grpc

RETRY_TIMEOUT = 5
HEARTBEAT_TIME = 10


def heartbeat(channel, terminate_event):
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)
    while not terminate_event.is_set():
        reply = stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
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


def run():
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

    try:
        # never returns unless there is an exception
        heartbeat_thread.join()
    except KeyboardInterrupt:
        terminate_event.set()
        channel.close()


if __name__ == "__main__":
    run()
