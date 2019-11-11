import numpy as np
from numproto import proto_to_ndarray

from xain.grpc import coordinator_pb2
from xain.grpc.coordinator import Coordinator


def test_rendezvous_accept():
    coordinator = Coordinator()
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.ACCEPT


def test_rendezvous_later():
    coordinator = Coordinator(required_participants=1)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer2")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_heartbeat_reply():
    # test that the coordinator replies with the correct state and round number
    coordinator = Coordinator()
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "peer1")

    assert isinstance(result, coordinator_pb2.HeartbeatReply)
    assert result.state == coordinator_pb2.State.STANDBY
    assert result.round == 0

    # update the round and state of the coordinator and check again
    coordinator.state = coordinator_pb2.State.ROUND
    coordinator.round = 10
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "peer1")

    assert result.state == coordinator_pb2.State.ROUND
    assert result.round == 10


def test_state_standby_round():
    # tests that the coordinator transitions from STANDBY to ROUND once enough participants
    # are connected
    coordinator = Coordinator(required_participants=1)

    assert coordinator.state == coordinator_pb2.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    assert coordinator.state == coordinator_pb2.State.ROUND
    assert coordinator.round == 1


def test_start_training():
    test_theta = [np.arange(10), np.arange(10, 20)]
    coordinator = Coordinator(required_participants=1, theta=test_theta)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    result = coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "peer1")
    received_theta = [proto_to_ndarray(nda) for nda in result.theta]

    np.testing.assert_equal(test_theta, received_theta)


def test_end_training():
    coordinator = Coordinator(required_participants=2)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer1")

    assert len(coordinator.theta_updates) == 1
    assert len(coordinator.histories) == 1
    assert len(coordinator.metricss) == 1


def test_end_training_round_update():
    # Test that the round number is updated once all participants send their updates
    coordinator = Coordinator(required_participants=2)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer2")

    # check that we are currently in round 1
    assert coordinator.round == 1

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer1")
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer2")

    # check that round number was updated
    assert coordinator.round == 2


def test_end_training_reinitialize_local_models():
    coordinator = Coordinator(required_participants=2)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer1")

    # After one participant sends its updates we should have one update in the coordinator
    assert len(coordinator.theta_updates) == 1
    assert len(coordinator.histories) == 1
    assert len(coordinator.metricss) == 1

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer2")

    # once the second participant delivers its updates the round ends and the local models
    # are reinitialized
    assert coordinator.theta_updates == []
    assert coordinator.histories == []
    assert coordinator.metricss == []


def test_training_finished():
    coordinator = Coordinator(required_participants=1, num_rounds=2)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    # Deliver results for 2 rounds
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer1")
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "peer1")

    assert coordinator.state == coordinator_pb2.State.FINISHED
