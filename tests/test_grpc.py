# TODO: https://xainag.atlassian.net/browse/XP-241 will break this test
# TODO: https://xainag.atlassian.net/browse/XP-373 will fix it again (please bear with us in the meantime)

import sys
import threading
from concurrent import futures
from unittest import mock

import grpc
import numpy as np
import pytest
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats

# TODO: https://xainag.atlassian.net/browse/XP-373 will fix this below
"""
from xain_fl.coordinator.legacy_participant import (
    StateRecord,
    end_training,
    message_loop,
    rendezvous,
    start_training,
)
"""
from xain_fl.coordinator.participants import Participants
from xain_proto.fl import (
    coordinator_pb2,
    coordinator_pb2_grpc,
    hellonumproto_pb2,
    hellonumproto_pb2_grpc,
)

# Some grpc tests fail on macos.
# `pytestmark` when defined on a module will mark all tests in that module.
# For more information check
# http://doc.pytest.org/en/latest/skipping.html#skip-all-test-functions-of-a-class-or-module
if sys.platform == "darwin":
    pytestmark = pytest.mark.xfail(reason="some grpc tests fail on macos")

# pylint: disable=W0613,W0621
@pytest.mark.integration
def test_greeter_server(greeter_server):
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = hellonumproto_pb2_grpc.NumProtoServerStub(channel)

        nda = np.arange(10)
        response = stub.SayHelloNumProto(
            hellonumproto_pb2.NumProtoRequest(arr=ndarray_to_proto(nda))
        )

        response_nda = proto_to_ndarray(response.arr)

        assert np.array_equal(nda * 2, response_nda)


@pytest.mark.integration
def test_participant_rendezvous_accept(participant_stub, coordinator_service):
    reply = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())

    assert reply.response == coordinator_pb2.RendezvousResponse.ACCEPT


# TODO(XP-119): Fix test so it also runs correctly on macos
@pytest.mark.integration
def test_participant_rendezvous_later(participant_stub):

    # populate participants
    coordinator = Coordinator(
        minimum_participants_in_round=10, fraction_of_participants=1.0
    )
    required_participants = 10
    for i in range(required_participants):
        coordinator.participants.add(str(i))

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator), server
    )
    server.add_insecure_port("localhost:50051")
    server.start()

    # try to rendezvous the 11th participant
    reply = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    server.stop(0)

    assert reply.response == coordinator_pb2.RendezvousResponse.LATER


@pytest.mark.integration
def test_heartbeat(participant_stub, coordinator_service):
    # first we need to rendezvous so that the participant is added to the list of participants
    _ = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    reply = participant_stub.Heartbeat(coordinator_pb2.HeartbeatRequest())

    # the Coordinator is initialised in conftest.py::coordinator_service with 10 participants
    # needed per round. so here we expect the HeartbeatReply to have State.STANDBY
    # because we connected only one participant
    assert reply == coordinator_pb2.HeartbeatReply()
    assert coordinator_service.coordinator.state == coordinator_pb2.State.STANDBY


@pytest.mark.integration
def test_heartbeat_denied(participant_stub, coordinator_service):
    # heartbeat requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
        assert reply.status_code == grpc.StatusCode.PERMISSION_DENIED


@mock.patch("threading.Event.is_set", side_effect=[False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain_fl.coordinator.coordinator.Coordinator.remove_participant")
def test_monitor_heartbeats(mock_participants_remove, _mock_sleep, _mock_event):
    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    coordinator = Coordinator()
    coordinator.participants = participants

    terminate_event = threading.Event()
    monitor_heartbeats(coordinator, terminate_event)

    mock_participants_remove.assert_called_once_with("participant_1")


@mock.patch("threading.Event.is_set", side_effect=[False, True])
@mock.patch("time.sleep", return_value=None)
def test_monitor_heartbeats_remove_participant(_mock_sleep, _mock_event):
    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    coordinator = Coordinator()
    coordinator.participants = participants

    terminate_event = threading.Event()
    monitor_heartbeats(coordinator, terminate_event)

    assert participants.len() == 0


# TODO: https://xainag.atlassian.net/browse/XP-373 will fix this below
"""
@mock.patch("threading.Event.is_set", side_effect=[False, False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain_proto.fl.coordinator_pb2.HeartbeatRequest")
def test_participant_heartbeat(mock_heartbeat_request, _mock_sleep, _mock_event):
    channel = mock.MagicMock()
    terminate_event = threading.Event()
    st = StateRecord()

    message_loop(channel, st, terminate_event)

    # check that the heartbeat is sent exactly twice
    mock_heartbeat_request.assert_has_calls([mock.call(), mock.call()])
"""

# TODO: https://xainag.atlassian.net/browse/XP-373 will fix this below
"""
@pytest.mark.skip("Skipping due to moving of the grpc participant as sdk to xain-sdk")
@pytest.mark.integration
def test_start_training(coordinator_service):
    test_weights = [np.arange(10), np.arange(10, 20)]

    # set coordinator global model and hyper-params so that it needs only 1 participant
    coord = coordinator_service.coordinator
    coord.minimum_participants_in_round = 1
    coord.fraction_of_participants = 1.0
    coord.epochs = 5
    coord.epoch_base = 2
    coord.weights = test_weights
    coord.minimum_connected_participants = coord.get_minimum_connected_participants()

    # simulate a participant communicating with coordinator via channel
    with grpc.insecure_channel("localhost:50051") as channel:
        # we need to rendezvous before we can send any other requests
        rendezvous(channel)
        # call startTraining service method on coordinator
        weights, epochs, epoch_base = start_training(channel)

    # check global model received
    assert epochs == 5
    assert epoch_base == 2
    np.testing.assert_equal(weights, test_weights)
"""


@pytest.mark.integration
def test_start_training_denied(participant_stub, coordinator_service):
    # start training requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.StartTraining(coordinator_pb2.StartTrainingRequest())
        assert reply.status_code == grpc.StatusCode.PERMISSION_DENIED


@pytest.mark.skip("Skipping due to moving of the grpc participant as sdk to xain-sdk")
@pytest.mark.integration
def test_start_training_failed_precondition(participant_stub, coordinator_service):
    # start training requests are only allowed if the coordinator is in ROUND state.
    # Since we need 10 participants to be connected (see conftest.py::coordinator_service)
    # the StartTrainingRequest is expected to fail
    participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.StartTraining(coordinator_pb2.StartTrainingRequest())
        assert reply.status_code == grpc.StatusCode.FAILED_PRECONDITION


# TODO: https://xainag.atlassian.net/browse/XP-373 will fix this below
"""
@pytest.mark.skip("Skipping due to moving of the grpc participant as sdk to xain-sdk")
@pytest.mark.integration
def test_end_training(coordinator_service):
    assert coordinator_service.coordinator.round.updates == {}

    # simulate trained local model data
    test_weights, number_samples = [np.arange(20, 30), np.arange(30, 40)], 2
    metrics = {"metric": [np.arange(10, 20), np.arange(5, 10)]}

    with grpc.insecure_channel("localhost:50051") as channel:
        # we first need to rendezvous before we can send any other request
        rendezvous(channel)
        # call endTraining service method on coordinator
        end_training(  # pylint: disable-msg=no-value-for-parameter
            channel, test_weights, number_samples, metrics
        )
    # check local model received...

    assert len(coordinator_service.coordinator.round.updates) == 1

    round_ = coordinator_service.coordinator.round

    # first the weights update
    _, update = round_.updates.popitem()
    tu1, tu2 = update["weight_update"]
    assert tu2 == number_samples
    np.testing.assert_equal(tu1, test_weights)

    m = update["metrics"]
    assert m.keys() == metrics.keys()
    for k, vals in metrics.items():
        np.testing.assert_allclose(m[k], vals)
"""


@pytest.mark.integration
def test_end_training_duplicated_updates(coordinator_service, participant_stub):
    # participant can only send updates once in a single round
    participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())

    participant_stub.EndTraining(coordinator_pb2.EndTrainingRequest())

    with pytest.raises(grpc.RpcError):
        reply = participant_stub.EndTraining(coordinator_pb2.EndTrainingRequest())
        assert reply.status_code == grpc.StatusCode.ALREADY_EXISTS


@pytest.mark.integration
def test_end_training_denied(participant_stub, coordinator_service):
    # heartbeat requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.EndTraining(coordinator_pb2.EndTrainingRequest())
        assert reply.status_code == grpc.StatusCode.PERMISSION_DENIED
