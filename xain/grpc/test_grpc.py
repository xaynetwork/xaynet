import sys
import threading
from concurrent import futures
from unittest import mock

import grpc
import numpy as np
import pytest
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.grpc import (
    coordinator_pb2,
    coordinator_pb2_grpc,
    hellonumproto_pb2,
    hellonumproto_pb2_grpc,
)
from xain.grpc.coordinator import Coordinator, Participants, monitor_heartbeats
from xain.grpc.participant import end_training, heartbeat, start_training

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


def mocked_init(self, participants, required_participants=10):
    """Sets `num_accepted_participants` to be the same as `required_participants` so that
    the coordinator tells the client to try later.
    """
    self.required_participants = required_participants

    # populate participants
    participants = Participants()
    for i in range(required_participants):
        participants.add(str(i))
    self.participants = participants


# TODO: Fix test so it also runs correctly on macos
@pytest.mark.integration
@mock.patch("xain.grpc.coordinator.Coordinator.__init__", new=mocked_init)
def test_participant_rendezvous_later(participant_stub):

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        Coordinator(Participants()), server
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


@mock.patch("threading.Event.is_set", side_effect=[False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain.grpc.coordinator.Participants.remove")
def test_monitor_heartbeats(mock_participants_remove, _mock_sleep, _mock_event):
    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    terminate_event = threading.Event()
    monitor_heartbeats(participants, terminate_event)

    mock_participants_remove.assert_called_once_with("participant_1")


@mock.patch("threading.Event.is_set", side_effect=[False, True])
@mock.patch("time.sleep", return_value=None)
def test_monitor_heartbeats_remove_participant(_mock_sleep, _mock_event):
    participants = Participants()
    participants.add("participant_1")
    participants.participants["participant_1"].heartbeat_expires = 0

    terminate_event = threading.Event()
    monitor_heartbeats(participants, terminate_event)

    assert participants.len() == 0


@mock.patch("threading.Event.is_set", side_effect=[False, False, True])
@mock.patch("time.sleep", return_value=None)
@mock.patch("xain.grpc.coordinator_pb2.HeartbeatRequest")
def test_participant_heartbeat(mock_heartbeat_request, _mock_sleep, _mock_event):
    channel = mock.MagicMock()
    terminate_event = threading.Event()
    selected_event = threading.Event()

    heartbeat(channel, terminate_event, selected_event)

    # check that the heartbeat is sent exactly twice
    mock_heartbeat_request.assert_has_calls([mock.call(), mock.call()])


@pytest.mark.integration
def test_start_training(coordinator_service):
    test_theta = [np.arange(10), np.arange(10, 20)]
    # set coordinator global model data
    coordinator_service.epochs = 5
    coordinator_service.epoch_base = 2
    coordinator_service.theta = test_theta
    # simulate a participant communicating with coordinator via channel
    with grpc.insecure_channel("localhost:50051") as channel:
        # call startTraining service method on coordinator
        theta, epochs, epoch_base = start_training(channel)
        # check global model received
        assert epochs == 5
        assert epoch_base == 2
        np.testing.assert_equal(theta, test_theta)


@pytest.mark.integration
def test_end_training(coordinator_service):
    assert not coordinator_service.theta_updates
    assert not coordinator_service.histories
    assert not coordinator_service.metricss
    # simulate trained local model data
    test_theta, num = [np.arange(20, 30), np.arange(30, 40)], 2
    his = {"aaa": [1.1, 2.1], "bbb": [3.1, 4.1]}
    mets = 1, [3, 4, 5]
    with grpc.insecure_channel("localhost:50051") as channel:
        # call endTraining service method on coordinator
        end_training(channel, (test_theta, num), his, mets)
    # check local model received...
    assert len(coordinator_service.theta_updates) == 1
    assert len(coordinator_service.histories) == 1
    assert len(coordinator_service.metricss) == 1
    # first the theta update
    tu1, tu2 = coordinator_service.theta_updates[0]
    assert tu2 == num
    np.testing.assert_equal(tu1, test_theta)
    # history values are *floats* so a naive assert == won't do
    h = coordinator_service.histories[0]
    assert h.keys() == his.keys()
    for k, vals in his.items():
        np.testing.assert_allclose(h[k], vals)
    # finally metrics
    assert coordinator_service.metricss[0] == mets
