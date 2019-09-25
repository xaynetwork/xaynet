from concurrent import futures

import grpc
import pytest

from xain.grpc import coordinator_pb2_grpc, hellonumproto_pb2_grpc
from xain.grpc.coordinator import Coordinator, Participants
from xain.grpc.numproto_server import NumProtoServer


@pytest.fixture
def greeter_server():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    hellonumproto_pb2_grpc.add_NumProtoServerServicer_to_server(
        NumProtoServer(), server
    )
    server.add_insecure_port("localhost:50051")
    server.start()
    yield
    server.stop(0)


@pytest.fixture
def coordinator_service():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        Coordinator(Participants()), server
    )
    server.add_insecure_port("localhost:50051")

    server.start()
    yield
    server.stop(0)


@pytest.fixture
def participant_stub():
    channel = grpc.insecure_channel("localhost:50051")
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    return stub
