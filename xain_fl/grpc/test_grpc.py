import sys
import threading
from concurrent import futures
from unittest import mock

import grpc
import numpy as np
import pytest
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import (
    coordinator_pb2,
    coordinator_pb2_grpc,
    hellonumproto_pb2,
    hellonumproto_pb2_grpc,
)
from xain_fl.grpc.coordinator import (
    Coordinator,
    CoordinatorGrpc,
    Participants,
    monitor_heartbeats,
)
from xain_fl.grpc.participant import (
    StateRecord,
    end_training,
    message_loop,
    rendezvous,
    start_training,
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


# TODO: Fix test so it also runs correctly on macos
@pytest.mark.integration
def test_participant_rendezvous_later(participant_stub):

    # populate participants
    coordinator = Coordinator()
    required_participants = 10
    for i in range(required_participants):
        coordinator.participants.add(str(i))

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator), server
    )
    server.add_insecure_port("localhost:50051")
    server.start()

    reply = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    server.stop(0)

    assert reply.response == coordinator_pb2.RendezvousResponse.LATER


@pytest.mark.integration
def test_heartbeat(participant_stub, coordinator_service):
    # first we need to rendezvous so that the participant is added to the list of participants
    _ = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    reply = participant_stub.Heartbeat(coordinator_pb2.HeartbeatRequest())

    assert reply == coordinator_pb2.HeartbeatReply()


@pytest.mark.integration
def test_heartbeat_denied(participant_stub, coordinator_service):
    # heartbeat requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.Heartbeat(coordinator_pb2.HeartbeatRequest())
        assert reply.status_code == grpc.StatusCode.PERMISSION_DENIED


@mock.patch("threading.Event.is_set", side_effect=[False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain_fl.grpc.coordinator.Coordinator.remove_participant")
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


@mock.patch("threading.Event.is_set", side_effect=[False, False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain_fl.grpc.coordinator_pb2.HeartbeatRequest")
def test_participant_heartbeat(mock_heartbeat_request, _mock_sleep, _mock_event):
    channel = mock.MagicMock()
    terminate_event = threading.Event()
    st = StateRecord()

    message_loop(channel, st, terminate_event)

    # check that the heartbeat is sent exactly twice
    mock_heartbeat_request.assert_has_calls([mock.call(), mock.call()])


@pytest.mark.integration
def test_start_training(coordinator_service):
    test_theta = [np.arange(10), np.arange(10, 20)]

    # set coordinator global model data
    coordinator_service.coordinator.required_participants = 1
    coordinator_service.coordinator.epochs = 5
    coordinator_service.coordinator.epoch_base = 2
    coordinator_service.coordinator.theta = test_theta

    # simulate a participant communicating with coordinator via channel
    with grpc.insecure_channel("localhost:50051") as channel:
        # we need to rendezvous before we can send any other requests
        rendezvous(channel)
        # call startTraining service method on coordinator
        theta, epochs, epoch_base = start_training(channel)

    # check global model received
    assert epochs == 5
    assert epoch_base == 2
    np.testing.assert_equal(theta, test_theta)


@pytest.mark.integration
def test_start_training_denied(participant_stub, coordinator_service):
    # start training requests are only allowed if the participant has already
    # rendezvous with the coordinator
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.StartTraining(coordinator_pb2.StartTrainingRequest())
        assert reply.status_code == grpc.StatusCode.PERMISSION_DENIED


@pytest.mark.integration
def test_start_training_failed_precondition(participant_stub, coordinator_service):
    # start training requests are only allowed if the coordinator is in the
    # ROUND state
    participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())
    with pytest.raises(grpc.RpcError):
        reply = participant_stub.StartTraining(coordinator_pb2.StartTrainingRequest())
        assert reply.status_code == grpc.StatusCode.FAILED_PRECONDITION


@pytest.mark.integration
def test_end_training(coordinator_service):
    assert coordinator_service.coordinator.round.updates == {}

    # simulate trained local model data
    test_theta, num = [np.arange(20, 30), np.arange(30, 40)], 2
    his = {"aaa": [1.1, 2.1], "bbb": [3.1, 4.1]}
    mets = 1, [3, 4, 5]

    with grpc.insecure_channel("localhost:50051") as channel:
        # we first need to rendezvous before we can send any other request
        rendezvous(channel)
        # call endTraining service method on coordinator
        end_training(channel, (test_theta, num), his, mets)
    # check local model received...

    assert len(coordinator_service.coordinator.round.updates) == 1

    round_ = coordinator_service.coordinator.round

    # first the theta update
    _, update = round_.updates.popitem()
    tu1, tu2 = update["theta_update"]
    assert tu2 == num
    np.testing.assert_equal(tu1, test_theta)

    # history values are *floats* so a naive assert == won't do
    h = update["history"]
    assert h.keys() == his.keys()
    for k, vals in his.items():
        np.testing.assert_allclose(h[k], vals)

    # finally metrics
    assert update["metrics"] == mets


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
