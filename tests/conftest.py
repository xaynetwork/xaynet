"""XAIN FL conftest for cproto"""

from concurrent import futures
import json
import threading

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
from xain_fl.coordinator.store import (
    AbstractGlobalWeightsWriter,
    AbstractLocalWeightsReader,
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
def s3_mock_stores():
    """
    Create a fake S3 store
    """

    s3_resource = MockS3Resource()
    participant_store = MockS3Participant(s3_resource)
    coordinator_store = MockS3Coordinator(s3_resource)
    return (coordinator_store, participant_store)


@pytest.fixture(scope="function")
def participant_store(s3_mock_stores):
    """Return an object the participants can use to read the global
    weights and write their local weights

    """
    return s3_mock_stores[1]


@pytest.fixture(scope="function")
def end_training_request(s3_mock_stores):
    """A fixture that returns a function that can be used to send an
    ``EndTrainingRequest`` to the coordinator.

    """
    participant_store = s3_mock_stores[1]

    def wrapped(
        coordinator: Coordinator,
        participant_id: str,
        round: int = 0,
        weights: ndarray = ndarray([]),
    ):
        """Write the local weights for the given round and the given
        participant, and send an ``EndTrainingRequest`` on behalf of
        that participant.

        """
        participant_store.write_weights(participant_id, round, weights)
        coordinator.on_message(
            EndTrainingRoundRequest(participant_id=participant_id), participant_id
        )

    return wrapped


@pytest.fixture(scope="function")
def coordinator(s3_mock_stores):
    """
    A function that instantiates a new coordinator.
    """
    store: MockS3Coordinator = s3_mock_stores[0]
    default_global_weights_writer: AbstractGlobalWeightsWriter = store
    default_local_weights_reader: AbstractLocalWeightsReader = store

    # pylint: disable=too-many-arguments
    def wrapped(
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
def json_participant_metrics_sample():
    """Return a valid participant metric object."""
    return json.dumps(
        [
            {
                "measurement": "participant",
                "time": 1582017483 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
            {
                "measurement": "participant",
                "time": 1582017484 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
        ]
    )


@pytest.fixture()
def coordinator_metrics_sample():
    """Return a valid coordinator metric object."""
    return {"state": 1, "round": 2, "number_of_selected_participants": 0}


@pytest.fixture
def coordinator_service(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator = coordinator(
        minimum_participants_in_round=10, fraction_of_participants=1.0
    )
    coordinator_grpc = CoordinatorGrpc(coordinator)
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(coordinator_grpc, server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield coordinator_grpc
    server.stop(0)


@pytest.fixture
def mock_coordinator_service(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    agg = ModelSumAggregator()
    ctrl = IdController()
    coordinator = coordinator(
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
