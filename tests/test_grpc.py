"""XAIN FL tests for gRPC coordinator"""

from concurrent import futures
import threading
import time
from typing import Callable, Dict, Generator, cast
from unittest import mock

import grpc
import numpy as np
import pytest
from xain_proto.fl.coordinator_pb2 import (
    EndTrainingRoundRequest,
    EndTrainingRoundResponse,
    HeartbeatRequest,
    HeartbeatResponse,
    RendezvousReply,
    RendezvousRequest,
    StartTrainingRoundRequest,
    StartTrainingRoundResponse,
    State,
)
from xain_proto.fl.coordinator_pb2_grpc import (
    CoordinatorStub,
    add_CoordinatorServicer_to_server,
)
from xain_sdk.config import Config
from xain_sdk.participant_state_machine import (
    StateRecord,
    end_training_round,
    message_loop,
    rendezvous,
    start_participant,
    start_training_round,
)

from tests.store import MockS3Coordinator, MockS3Participant
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats
from xain_fl.coordinator.participants import ParticipantContext, Participants


@pytest.fixture
def participant_config() -> Dict:
    """Create a valid participant config.

    Returns:
        A participant configuration.
    """

    return {
        "coordinator": {
            "host": "localhost",
            "port": 50051,
            "grpc_options": {
                "grpc.max_receive_message_length": -1,
                "grpc.max_send_message_length": -1,
            },
        },
        "storage": {
            "endpoint": "http://localhost:9000",
            "bucket": "aggregated_weights",
            "secret_access_key": "my-secret",
            "access_key_id": "my-key-id",
        },
        "logging": {"level": "info"},
    }


@pytest.mark.integration
def test_participant_rendezvous_accept(  # pylint: disable=unused-argument
    participant_stub: Generator, coordinator_service: Generator
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    response = cast(CoordinatorStub, participant_stub).Rendezvous(RendezvousRequest())

    assert response.reply == RendezvousReply.ACCEPT


@pytest.mark.integration
def test_participant_rendezvous_later(
    coordinator: Callable, participant_stub: Generator
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        coordinator: A fixture to create a new coordinator.
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
    """

    # populate participants
    coord = coordinator(minimum_participants_in_round=10, fraction_of_participants=1.0)
    required_participants = 10
    for i in range(required_participants):
        coord.participants.add(str(i))

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    add_CoordinatorServicer_to_server(CoordinatorGrpc(coord), server)
    server.add_insecure_port("localhost:50051")
    server.start()

    # try to rendezvous the 11th participant
    response = cast(CoordinatorStub, participant_stub).Rendezvous(RendezvousRequest())
    server.stop(0)

    assert response.reply == RendezvousReply.LATER


@pytest.mark.integration
def test_heartbeat(participant_stub: Generator, coordinator_service: Generator):
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    # first we need to rendezvous so that the participant is added to the list of participants
    _ = cast(CoordinatorStub, participant_stub).Rendezvous(RendezvousRequest())
    response = cast(CoordinatorStub, participant_stub).Heartbeat(HeartbeatRequest())

    # the Coordinator is initialised in conftest.py::coordinator_service with 10 participants
    # needed per round. so here we expect the HeartbeatResponse to have State.STANDBY
    # because we connected only one participant
    assert response == HeartbeatResponse()
    assert cast(CoordinatorGrpc, coordinator_service).coordinator.state == State.STANDBY


@pytest.mark.integration
def test_heartbeat_denied(  # pylint: disable=unused-argument
    participant_stub: Generator, coordinator_service: Generator
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    # heartbeat requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        response = cast(CoordinatorStub, participant_stub).Heartbeat(HeartbeatRequest())
        assert response.status_code == grpc.StatusCode.PERMISSION_DENIED


def test_monitor_heartbeats(mocker: mock.MagicMock, coordinator: Callable) -> None:
    """Test heartbeat monitoring.

    Test that when there is a participant with an expired heartbeat,
    ``Coordinator.remove_participant`` is called exactly once.

    Args:
        mocker: A magic mocker.
        coordinator: A fixture to create a new coordinator.
    """

    mock_remove_participant = mocker.patch(
        "xain_fl.coordinator.heartbeat.Coordinator.remove_participant"
    )
    mocker.patch(
        "xain_fl.coordinator.heartbeat.threading.Event.wait", return_value=None
    )
    mocker.patch(
        "xain_fl.coordinator.heartbeat.threading.Event.is_set",
        side_effect=[False, True],
    )

    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    coord = coordinator()
    coord.participants = participants

    terminate_event = threading.Event()
    monitor_heartbeats(coord, terminate_event)

    mock_remove_participant.assert_called_once_with("participant_1")


def test_monitor_heartbeats_remove_participant(
    coordinator: Callable, mocker: mock.MagicMock
) -> None:
    """Test heartbeat monitoring.

    Test that when the coordinator has exactly one participant with an expired
    heartbeat, it is removed correctly.

    Args:
        coordinator: A fixture to create a new coordinator.
        mocker: A magic mocker.
    """

    mocker.patch(
        "xain_fl.coordinator.heartbeat.threading.Event.is_set",
        side_effect=[False, True],
    )
    mocker.patch(
        "xain_fl.coordinator.heartbeat.threading.Event.wait", return_value=None
    )

    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    coord = coordinator()
    coord.participants = participants

    terminate_event = threading.Event()
    monitor_heartbeats(coord, terminate_event)

    assert participants.len() == 0


@pytest.mark.slow
def test_many_heartbeats_expire_in_short_interval(coordinator: Callable) -> None:
    """Test heartbeat monitoring.

    Make sure that heartbeat_monitor() works correctly under heavy load. This test was
    added to reproduce PB-104.

    Args:
        coordinator: A fixture to create a new coordinator.
    """

    participants = {}
    for i in range(0, 100):
        participant = ParticipantContext(str(i), 10, 5)
        participant.heartbeat_expires = time.time() + 0.1 + i / 1000
        participants[str(i)] = participant
    coord = coordinator()
    coord.participants.participants = participants

    terminate_event = threading.Event()

    def terminate_heartbeats_monitor():
        time.sleep(0.2)
        terminate_event.set()

    supervisor = threading.Thread(target=terminate_heartbeats_monitor, daemon=True)
    supervisor.start()

    monitor_heartbeats(coord, terminate_event)

    supervisor.join(timeout=0.1)
    assert not participants


@mock.patch(
    "xain_sdk.participant_state_machine.threading.Event.is_set",
    side_effect=[False, False, True],
)
@mock.patch(
    "xain_sdk.participant_state_machine.threading.Event.wait", return_value=None
)
@mock.patch("xain_sdk.participant_state_machine.HeartbeatRequest")
def test_message_loop(
    mock_heartbeat_request: mock.MagicMock,
    _mock_sleep: mock.MagicMock,
    _mock_event: mock.MagicMock,
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        mock_heartbeat_request: A magic mock for the heartbeat.
        _mock_sleep: A magic mock for the waiting event.
        _mock_event: A magic mock for the terminate event.
    """

    channel = mock.MagicMock()
    terminate_event = threading.Event()
    state_record = StateRecord()
    participant_id = "123"

    message_loop(channel, participant_id, state_record, terminate_event)

    # check that the heartbeat is sent exactly twice
    expected_call = mock.call(round=-1, state=State.READY)
    mock_heartbeat_request.assert_has_calls([expected_call, expected_call])


@pytest.mark.integration
def test_start_training_round(coordinator_service: Generator):
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        coordinator_service: [description].
    """

    test_weights = np.arange(10)

    # set coordinator global model and hyper-params so that it needs only 1 participant
    coord = cast(CoordinatorGrpc, coordinator_service).coordinator
    coord.minimum_participants_in_round = 1
    coord.fraction_of_participants = 1.0
    coord.epochs = 5
    coord.epoch_base = 2
    coord.weights = test_weights
    coord.minimum_connected_participants = coord.get_minimum_connected_participants()

    # simulate a participant communicating with coordinator via channel
    with grpc.insecure_channel("localhost:50051") as channel:
        # we need to rendezvous before we can send any other requests
        rendezvous(channel, participant_id="123")
        # call StartTrainingRound service method on coordinator
        epochs, epoch_base = start_training_round(channel, participant_id="123")

    # check global model received
    assert epochs == 5
    assert epoch_base == 2


@pytest.mark.integration
def test_start_training_round_denied(  # pylint: disable=unused-argument
    participant_stub: Generator, coordinator_service: Generator
):
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    # start training requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        response = cast(CoordinatorStub, participant_stub).StartTrainingRound(
            StartTrainingRoundRequest()
        )
        assert response.status_code == grpc.StatusCode.PERMISSION_DENIED


@pytest.mark.integration
def test_start_training_round_failed_precondition(  # pylint: disable=unused-argument
    participant_stub: Generator, coordinator_service: Generator
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    # start training requests are only allowed if the coordinator is in ROUND state.
    # Since we need 10 participants to be connected (see conftest.py::coordinator_service)
    # the StartTrainingRoundRequest is expected to fail
    cast(CoordinatorStub, participant_stub).Rendezvous(RendezvousRequest())
    with pytest.raises(grpc.RpcError):
        response = cast(CoordinatorStub, participant_stub).StartTrainingRound(
            StartTrainingRoundRequest()
        )
        assert response.status_code == grpc.StatusCode.FAILED_PRECONDITION


@pytest.mark.integration
def test_end_training_round(
    coordinator_service: Generator,
    json_participant_metrics_sample: str,
    participant_store: MockS3Participant,
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        coordinator_service: [description].
        json_participant_metrics_sample: A valid participant metric object.
        participant_store: A mocked S3 store for the participant.
    """

    assert cast(CoordinatorGrpc, coordinator_service).coordinator.round.updates == {}

    # simulate trained local model data
    test_weights = np.arange(20)
    number_samples = 2

    with grpc.insecure_channel("localhost:50051") as channel:
        # we first need to rendezvous before we can send any other request
        rendezvous(channel, participant_id="123")
        # call EndTrainingRound service method on coordinator
        participant_store.write_weights("participant1", 0, test_weights)
        end_training_round(
            channel, "participant1", number_samples, json_participant_metrics_sample
        )
    # check local model received...

    assert (
        len(cast(CoordinatorGrpc, coordinator_service).coordinator.round.updates) == 1
    )

    round_ = cast(CoordinatorGrpc, coordinator_service).coordinator.round

    # first the weights update
    _, round_update = round_.updates.popitem()
    np.testing.assert_equal(round_update["model_weights"], test_weights)
    assert round_update["aggregation_data"] == number_samples


@pytest.mark.integration
def test_end_training_round_duplicated_updates(  # pylint: disable=unused-argument
    coordinator_service: Generator,
    participant_stub: Generator,
    participant_store: MockS3Participant,
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        coordinator_service: [description].
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        participant_store: A mocked S3 store for the participant.
    """

    # participant can only send updates once in a single round
    cast(CoordinatorStub, participant_stub).Rendezvous(RendezvousRequest())

    participant_store.write_weights("participant1", 0, np.ndarray([]))
    cast(CoordinatorStub, participant_stub).EndTrainingRound(
        EndTrainingRoundRequest(participant_id="participant1")
    )

    with pytest.raises(grpc.RpcError):
        response = cast(CoordinatorStub, participant_stub).EndTrainingRound(
            EndTrainingRoundRequest()
        )
        assert response.status_code == grpc.StatusCode.ALREADY_EXISTS


@pytest.mark.integration
def test_end_training_round_denied(  # pylint: disable=unused-argument
    participant_stub: Generator, coordinator_service: Generator
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
    """

    # heartbeat requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        response = cast(CoordinatorStub, participant_stub).EndTrainingRound(
            EndTrainingRoundRequest()
        )
        assert response.status_code == grpc.StatusCode.PERMISSION_DENIED


@pytest.mark.integration
def test_full_training_round(
    participant_stubs: Generator,
    coordinator_service: Generator,
    participant_store: MockS3Participant,
) -> None:
    """Run a complete training round with multiple participants.

    Args:
        participant_stub: A fixture to create a local coordinator gRPC stub for a
            participant.
        coordinator_service: [description].
        participant_store: A mocked S3 store for the participant.
    """

    weights = np.ndarray([1, 2, 3, 4])
    cast(CoordinatorGrpc, coordinator_service).coordinator.weights = weights

    # Create 10 partipants
    participants = [next(participant_stubs) for _ in range(0, 10)]

    # 9 participants out of 10 connect to the coordinator, so it stays
    # in STANDBY and accepts the connections.
    for participant in participants[:-1]:
        response = participant.Rendezvous(RendezvousRequest())
        assert response.reply == RendezvousReply.ACCEPT

        response = participant.Heartbeat(HeartbeatRequest())
        assert response == HeartbeatResponse(state=State.STANDBY, round=0)

    assert cast(CoordinatorGrpc, coordinator_service).coordinator.state == State.STANDBY
    assert cast(CoordinatorGrpc, coordinator_service).coordinator.current_round == 0
    assert cast(CoordinatorGrpc, coordinator_service).coordinator.epoch_base == 0

    # The 10th participant connects, so the coordinator switches to ROUND
    last_participant = participants[-1]
    response = last_participant.Rendezvous(RendezvousRequest())
    assert response.reply == RendezvousReply.ACCEPT

    assert cast(CoordinatorGrpc, coordinator_service).coordinator.state == State.ROUND
    assert cast(CoordinatorGrpc, coordinator_service).coordinator.current_round == 0
    assert cast(CoordinatorGrpc, coordinator_service).coordinator.epoch_base == 0

    response = last_participant.Heartbeat(HeartbeatRequest())
    assert response == HeartbeatResponse(state=State.ROUND, round=0)

    # The initial 9 participants send another heartbeat request.
    for participant in participants[:-1]:
        response = participant.Heartbeat(HeartbeatRequest(state=State.STANDBY, round=0))
    assert response == HeartbeatResponse(state=State.ROUND, round=0)

    # The participants start training
    for participant in participants:
        response = participant.StartTrainingRound(StartTrainingRoundRequest())
        assert response == StartTrainingRoundResponse(
            epochs=cast(CoordinatorGrpc, coordinator_service).coordinator.epochs,
            epoch_base=cast(
                CoordinatorGrpc, coordinator_service
            ).coordinator.epoch_base,
        )

    # The first 9 participants end training
    for (i, participant) in enumerate(participants[:-1]):
        participant_id = f"participant{i}"
        participant_store.write_weights(participant_id, 0, weights)
        response = participant.EndTrainingRound(
            EndTrainingRoundRequest(participant_id=participant_id, number_samples=1)
        )
        assert response == EndTrainingRoundResponse()
        cast(
            MockS3Coordinator,
            cast(CoordinatorGrpc, coordinator_service).coordinator.local_weights_reader,
        ).assert_read(participant_id, 0)

    assert not cast(
        CoordinatorGrpc, coordinator_service
    ).coordinator.round.is_finished()
    cast(
        MockS3Coordinator,
        cast(CoordinatorGrpc, coordinator_service).coordinator.global_weights_writer,
    ).assert_didnt_write(1)

    # The last participant finishes training
    participant_id = f"participant9"
    participant_store.write_weights(participant_id, 0, weights)
    response = last_participant.EndTrainingRound(
        EndTrainingRoundRequest(participant_id=participant_id, number_samples=1)
    )
    assert response == EndTrainingRoundResponse()

    # Make sure we wrote the results for the given round
    cast(
        MockS3Coordinator,
        cast(CoordinatorGrpc, coordinator_service).coordinator.global_weights_writer,
    ).assert_wrote(1, cast(CoordinatorGrpc, coordinator_service).coordinator.weights)


@pytest.mark.integration
@pytest.mark.slow
def test_start_participant(  # pylint: disable=redefined-outer-name
    mock_coordinator_service: Generator,
    participant_config: Dict,
    mocker: mock.MagicMock,
    participant_store: MockS3Participant,
) -> None:
    """[summary]

    .. todo:: PB-50: Advance docstrings.

    Args:
        mock_coordinator_service: A fixture to create a local coordinator gRPC service.
        participant_config: A valid participant configuration.
        mocker: A magic mocker.
        participant_store: A mocked S3 store for the participant.
    """

    init_weight = np.arange(10)
    cast(CoordinatorGrpc, mock_coordinator_service).coordinator.weights = init_weight

    # pylint: disable=missing-docstring
    def mock_participant_store(*_args):
        return participant_store

    # mock a local participant with a constant train_round function
    mocker.patch(
        "xain_sdk.participant_state_machine.S3LocalWeightsWriter",
        new=mock_participant_store,
    )
    mocker.patch(
        "xain_sdk.participant_state_machine.S3GlobalWeightsReader",
        new=mock_participant_store,
    )
    mock_local_part = mocker.patch("xain_sdk.participant.Participant")
    mock_local_part.init_weights.return_value = init_weight
    mock_local_part.train_round.return_value = init_weight, 1
    mock_local_part.dummy_id = "participant1"

    config: Config = Config.from_unchecked_dict(participant_config)

    start_participant(mock_local_part, config)

    coord = cast(CoordinatorGrpc, mock_coordinator_service).coordinator
    assert coord.state == State.FINISHED

    # coordinator set to 2 rounds for good measure, but the resulting
    # aggregated weights are the same as a single round
    assert coord.current_round == 1

    # expect weight aggregated by summation - see mock_coordinator_service
    np.testing.assert_equal(coord.weights, init_weight)
