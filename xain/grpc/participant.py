import grpc
from google.protobuf import empty_pb2

from xain.grpc import coordinator_pb2, coordinator_pb2_grpc


def run():
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = coordinator_pb2_grpc.CoordinatorStub(channel)

        reply = stub.Rendezvous(empty_pb2.Empty())
        if reply.response == coordinator_pb2.RendezvousResponse.ACCEPT:
            print("Participant received: ACCEPT")
        elif reply.response == coordinator_pb2.RendezvousResponse.LATER:
            print("Participant received: LATER")


if __name__ == "__main__":
    run()
