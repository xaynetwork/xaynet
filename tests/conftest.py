"""XAIN FL conftest for cproto"""

from concurrent import futures

import grpc
import pytest
from xain_proto.fl import coordinator_pb2_grpc, hellonumproto_pb2_grpc

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.helloproto.numproto_server import NumProtoServer

from .port_forwarding import ConnectionManager
from .store import TestStore


@pytest.fixture
def greeter_server():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
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

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator = Coordinator(minimum_participants_in_round=10, fraction_of_participants=1.0)
    store = TestStore()
    coordinator_grpc = CoordinatorGrpc(coordinator, store)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield coordinator_grpc
    server.stop(0)


@pytest.fixture
def participant_stub():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Returns:
        [type]: [description]
    """

    channel = grpc.insecure_channel("localhost:50051")
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    yield stub

    channel.close()


def port_generator():
    """A generator that yields incrementing port numbers.

    """
    port = 50052
    while True:
        yield port
        port += 1


@pytest.fixture
def participant_stubs():
    """Generator that yields functions instantiate participant stubs.
    Each participant creates a new TCP connection, so that they get a
    different participant ID.

    """

    ports = port_generator()
    connections = ConnectionManager()
    channels = []

    def generate_participant_stubs():
        for port in ports:
            connections.start("localhost", port, "localhost", 50051)
            channel = grpc.insecure_channel(f"localhost:{port}")
            channels.append(channel)
            stub = coordinator_pb2_grpc.CoordinatorStub(channel)
            yield stub

    yield generate_participant_stubs()

    for channel in channels:
        channel.close()

    connections.stop_all()
