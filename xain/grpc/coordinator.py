import time
from concurrent import futures

import grpc

from xain.grpc import coordinator_pb2, coordinator_pb2_grpc

_ONE_DAY_IN_SECONDS = 60 * 60 * 24


class Coordinator(coordinator_pb2_grpc.CoordinatorServicer):
    def __init__(self, required_participants=10):
        self.required_participants = required_participants
        self.num_accepted_participants = 0

    def Rendezvous(self, request, context):
        if self.num_accepted_participants < self.required_participants:
            response = coordinator_pb2.RendezvousResponse.ACCEPT
            self.num_accepted_participants += 1
            print(
                f"Accepted participant {context.peer()}"
                f" # participants: {self.num_accepted_participants}"
            )
        else:
            response = coordinator_pb2.RendezvousResponse.LATER
            print(
                f"Rejected participant {context.peer()}"
                f" # participants: {self.num_accepted_participants}"
            )

        return coordinator_pb2.RendezvousReply(response=response)


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(Coordinator(), server)
    server.add_insecure_port("[::]:50051")
    server.start()

    print("Coordinator waiting for connections...")

    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        server.stop(0)


if __name__ == "__main__":
    serve()
