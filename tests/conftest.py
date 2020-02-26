"""XAIN FL conftest for cproto"""

from concurrent import futures
import json
import threading
from typing import Callable, Dict, Generator, Tuple

import grpc
import numpy as np
from numpy import ndarray
import pytest
from xain_proto.fl import coordinator_pb2_grpc
from xain_proto.fl.coordinator_pb2 import EndTrainingRoundRequest

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats
from xain_fl.coordinator.metrics_store import (
    AbstractMetricsStore,
    NullObjectMetricsStore,
)
from xain_fl.fl.coordinator.aggregate import (
    Aggregator,
    ModelSumAggregator,
    WeightedAverageAggregator,
)
from xain_fl.fl.coordinator.controller import Controller, IdController, RandomController

from .port_forwarding import ConnectionManager
from .store import MockS3Coordinator, MockS3Participant, MockS3Resource

# pylint: disable=redefined-outer-name


@pytest.fixture(scope="function")
def s3_mock_stores() -> Tuple[MockS3Coordinator, MockS3Participant]:
    """Create fake S3 stores.

    Returns:
        A mocked store for the coordinator and participant.
    """

    s3_resource = MockS3Resource()
    participant_store = MockS3Participant(s3_resource)
    coordinator_store = MockS3Coordinator(s3_resource)
    return (coordinator_store, participant_store)


@pytest.fixture(scope="function")
def participant_store(
    s3_mock_stores: Tuple[MockS3Coordinator, MockS3Participant]
) -> MockS3Participant:
    """Create a fake S3 store for the participant.

    Args:
        s3_mock_stores: The mocked S3 stores.

    Returns:
        The mocked S3 store for the participant.
    """

    return s3_mock_stores[1]


@pytest.fixture(scope="function")
def end_training_request(
    s3_mock_stores: Tuple[MockS3Coordinator, MockS3Participant]
) -> Callable:
    """A fixture to send an EndTrainingRoundRequest to the coordinator.

    Write the local weights for the given round and the given participant, and send an
    EndTrainingRequest on behalf of that participant.

    Args:
        s3_mock_stores: The mocked S3 stores.

    Returns:
        A function that can be used to send and EndTrainingRequest to the coordinator.
    """

    participant_store = s3_mock_stores[1]

    def wrapped(
        coordinator: Coordinator,
        participant_id: str,
        round: int = 0,
        weights: ndarray = ndarray([]),
    ):
        participant_store.write_weights(participant_id, round, weights)
        coordinator.on_message(
            EndTrainingRoundRequest(participant_id=participant_id), participant_id
        )

    return wrapped


@pytest.fixture(scope="function")
def coordinator(
    s3_mock_stores: Tuple[MockS3Coordinator, MockS3Participant]
) -> Callable:
    """Instantiate a new coordinator.

    Args:
        s3_mock_stores: The mocked S3 stores.

    Returns:
        A function to create a new coordinator.
    """

    store: MockS3Coordinator = s3_mock_stores[0]
    default_global_weights_writer: MockS3Coordinator = store
    default_local_weights_reader: MockS3Coordinator = store

    def wrapped(  # pylint: disable=too-many-arguments
        global_weights_writer=default_global_weights_writer,
        local_weights_reader=default_local_weights_reader,
        metrics_store: AbstractMetricsStore = NullObjectMetricsStore(),
        num_rounds: int = 1,
        minimum_participants_in_round: int = 1,
        fraction_of_participants: float = 1.0,
        weights: ndarray = np.empty(shape=(0,)),
        epochs: int = 1,
        epoch_base: int = 0,
        aggregator: Aggregator = WeightedAverageAggregator(),
        controller: Controller = RandomController(),
    ):
        return Coordinator(
            global_weights_writer,
            local_weights_reader,
            metrics_store=metrics_store,
            num_rounds=num_rounds,
            minimum_participants_in_round=minimum_participants_in_round,
            fraction_of_participants=fraction_of_participants,
            weights=weights,
            epochs=epochs,
            epoch_base=epoch_base,
            aggregator=aggregator,
            controller=controller,
        )

    return wrapped


@pytest.fixture()
def json_participant_metrics_sample() -> str:
    """Create a valid participant metric object.

    Returns:
        The metric sample as a JSON string.
    """

    return json.dumps(
        [
            {
                "measurement": "participant",
                "time": 1582017483 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00},
            },
            {
                "measurement": "participant",
                "time": 1582017484 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00},
            },
        ]
    )


@pytest.fixture()
def coordinator_metrics_sample() -> Dict:
    """Create a valid coordinator metric object.

    Returns:
        The metric sample as a dictionary.
    """

    return {"state": 1, "round": 2, "number_of_selected_participants": 0}


@pytest.fixture
def coordinator_service(coordinator: Callable) -> Generator:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        coordinator: A function to create a coordinator.

    Returns:
        [description].
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coord = coordinator(minimum_participants_in_round=10, fraction_of_participants=1.0)
    coordinator_grpc = CoordinatorGrpc(coord)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield coordinator_grpc
    server.stop(0)


@pytest.fixture
def mock_coordinator_service(coordinator: Callable) -> Generator:
    """Create a local coordinator gRPC service.

    Args:
        coordinator: A function to create a coordinator.

    Returns:
        A generated coordinator service.
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    agg = ModelSumAggregator()
    ctrl = IdController()
    coord = coordinator(
        num_rounds=2,
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
        aggregator=agg,
        controller=ctrl,
    )
    coordinator_grpc = CoordinatorGrpc(coord)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(coord, terminate_event)
    )
    monitor_thread.start()
    yield coordinator_grpc
    terminate_event.set()
    monitor_thread.join()
    server.stop(0)


@pytest.fixture
def participant_stub() -> Generator:
    """Create a local coordinator gRPC stub for a participant.

    Returns:
        A generated coordinator stub.
    """

    channel = grpc.insecure_channel("localhost:50051")
    stub = coordinator_pb2_grpc.CoordinatorStub(channel)

    yield stub

    channel.close()


def port_generator() -> Generator:
    """A generator that yields incrementing port numbers.

    Returns:
        The generator for incrementing port numbers.
    """

    port = 50052
    while True:
        yield port
        port += 1


@pytest.fixture
def participant_stubs() -> Generator:
    """Generator that yields functions instantiate participant stubs.

    Each participant creates a new TCP connection, so that they get a
    different participant ID.

    Returns:
        The generator for participant stubs.
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
