from xain.grpc import coordinator_pb2
from xain.grpc.coordinator import Coordinator


def test_rendezvous_accept():
    coordinator = Coordinator()
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    assert type(result) == coordinator_pb2.RendezvousReply
    assert result.response == coordinator_pb2.RendezvousResponse.ACCEPT


def test_rendezvous_later():
    coordinator = Coordinator(required_participants=1)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer2")

    assert type(result) == coordinator_pb2.RendezvousReply
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_heartbeat_reply():
    # test that the coordinator replies with the correct state and round number
    coordinator = Coordinator()
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "peer1")

    assert type(result) == coordinator_pb2.HeartbeatReply
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
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "peer1")

    assert coordinator.state == coordinator_pb2.State.ROUND
