"""XAIN FL conftest for cproto"""

from concurrent import futures

import grpc
import pytest

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.helloproto.numproto_server import NumProtoServer
from xain_proto.fl import coordinator_pb2_grpc, hellonumproto_pb2_grpc


@pytest.fixture
def greeter_server():
    """[summary]

    [extended_summary]
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    hellonumproto_pb2_grpc.add_NumProtoServerServicer_to_server(NumProtoServer(), server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield
    server.stop(0)


@pytest.fixture
def coordinator_service():
    """[summary]

    [extended_summary]
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator = Coordinator(minimum_participants_in_round=10, fraction_of_participants=1.0)
    coordinator_grpc = CoordinatorGrpc(coordinator)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield coordinator_grpc
    server.stop(0)


@pytest.fixture
def participant_stub():
    """[summary]

    [extended_summary]

    Returns:
        [type]: [description]
    """

    channel = grpc.insecure_channel("localhost:50051")
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    return stub
