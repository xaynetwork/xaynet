import time

import grpc

from xain.grpc import coordinator_pb2, coordinator_pb2_grpc

RETRY_TIMEOUT = 5


def run():
    with grpc.insecure_channel("localhost:50051") as channel:
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


if __name__ == "__main__":
    run()
