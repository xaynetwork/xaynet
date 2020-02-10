"""XAIN FL conftest for cproto"""

from concurrent import futures
import json
import threading

import grpc
import pytest
from xain_proto.fl import coordinator_pb2_grpc

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats
from xain_fl.fl.coordinator.aggregate import ModelSumAggregator
from xain_fl.fl.coordinator.controller import IdController

from .port_forwarding import ConnectionManager


@pytest.fixture()
def metrics_sample():
    """Return a valid metric object."""
    return json.dumps(
        [
            {
                "measurement": "CPU utilization",
                "time": 1234326435,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
            {
                "measurement": "CPU utilization",
                "time": 3542626236,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
        ]
    )


@pytest.fixture
def coordinator_service():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator = Coordinator(
        minimum_participants_in_round=10, fraction_of_participants=1.0
    )
    coordinator_grpc = CoordinatorGrpc(coordinator)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield coordinator_grpc
    server.stop(0)


@pytest.fixture
def mock_coordinator_service():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    agg = ModelSumAggregator()
    ctrl = IdController()
    coordinator = Coordinator(
        num_rounds=2,
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
        aggregator=agg,
        controller=ctrl,
    )
    coordinator_grpc = CoordinatorGrpc(coordinator)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(coordinator, terminate_event)
    )
    monitor_thread.start()
    yield coordinator_grpc
    terminate_event.set()
    monitor_thread.join()
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
