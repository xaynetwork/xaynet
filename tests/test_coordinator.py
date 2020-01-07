"""XAIN FL tests for coordinator"""

from numproto import proto_to_ndarray
import numpy as np
import pytest
from xain_proto.fl import coordinator_pb2
from xain_proto.fl.coordinator_pb2 import RendezvousReply, RendezvousRequest, RendezvousResponse

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.tools.exceptions import (
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)


def test_rendezvous_accept():
    """[summary]

    [extended_summary]
    """

    coordinator: Coordinator = Coordinator()
    result: RendezvousReply = coordinator.on_message(RendezvousRequest(), "participant1")

    assert isinstance(result, RendezvousReply)
    assert result.response == RendezvousResponse.ACCEPT


def test_rendezvous_later_fraction_1():
    """[summary]

    [extended_summary]
    """

    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_rendezvous_later_fraction_05():
    """[summary]

    [extended_summary]
    """

    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=0.5)

    # with 0.5 fraction it needs to accept at least two participants
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.ACCEPT

    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.ACCEPT

    # the third participant must receive LATER RendezvousResponse
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant3")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_coordinator_state_standby_round():
    """[summary]

    [extended_summary]
    """

    # tests that the coordinator transitions from STANDBY to ROUND once enough participants
    # are connected
    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=1.0)

    assert coordinator.state == coordinator_pb2.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND
    assert coordinator.current_round == 1


def test_start_training():
    """[summary]

    [extended_summary]
    """

    test_weights = [np.arange(10), np.arange(10, 20)]
    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0, weights=test_weights,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    result = coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "participant1")
    received_weights = [proto_to_ndarray(nda) for nda in result.weights]

    np.testing.assert_equal(test_weights, received_weights)


def start_training_wrong_state():
    """[summary]

    [extended_summary]
    """

    # if the coordinator receives a StartTraining request while not in the
    # ROUND state it will raise an exception
    coordinator = Coordinator(minimum_participants_in_round=2, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    with pytest.raises(InvalidRequestError):
        coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "participant1")


def test_end_training():
    """[summary]

    [extended_summary]
    """

    # we need two participants so that we can check the status of the local update mid round
    # with only one participant it wouldn't work because the local updates state is cleaned at
    # the end of each round
    coordinator = Coordinator(minimum_participants_in_round=2, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    assert len(coordinator.round.updates) == 1


def test_end_training_round_update():
    """[summary]

    [extended_summary]
    """

    # Test that the round number is updated once all participants sent their updates
    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    # check that we are currently in round 1
    assert coordinator.current_round == 1

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")
    # check we are still in round 1
    assert coordinator.current_round == 1
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant2")

    # check that round number was updated
    assert coordinator.current_round == 2


def test_end_training_reinitialize_local_models():
    """[summary]

    [extended_summary]
    """

    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    # After one participant sends its updates we should have one update in the coordinator
    assert len(coordinator.round.updates) == 1

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant2")

    # once the second participant delivers its updates the round ends and the local models
    # are reinitialized
    assert coordinator.round.updates == {}


def test_training_finished():
    """[summary]

    [extended_summary]
    """

    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    # Deliver results for 2 rounds
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.FINISHED


def test_wrong_participant():
    """[summary]

    [extended_summary]
    """

    # coordinator should not accept requests from participants that it has not accepted
    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant2")


def test_duplicated_update_submit():
    """[summary]

    [extended_summary]
    """

    # the coordinator should not accept multiples updates from the same participant
    # in the same round
    coordinator = Coordinator(minimum_participants_in_round=2, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    with pytest.raises(DuplicatedUpdateError):
        coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")


def test_remove_participant():
    """[summary]

    [extended_summary]
    """

    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=1.0)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND

    coordinator.remove_participant("participant1")

    assert coordinator.participants.len() == 0
    assert coordinator.state == coordinator_pb2.State.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND


def test_number_of_selected_participants():
    """[summary]

    [extended_summary]
    """

    # test that the coordinator needs minimum 3 participants and selects 2 of them
    coordinator = Coordinator(minimum_participants_in_round=2, fraction_of_participants=0.6)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    # the coordinator should wait for three participants to be connected before starting a round,
    # and select participants. Before that coordinator.round.participant_ids is an empty list
    assert coordinator.minimum_connected_participants == 3
    assert coordinator.state == coordinator_pb2.State.STANDBY
    assert coordinator.round.participant_ids == []

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    assert coordinator.state == coordinator_pb2.State.STANDBY
    assert coordinator.round.participant_ids == []

    # add the third participant
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant3")

    # now the coordinator must have started a round and selected 2 participants
    assert coordinator.state == coordinator_pb2.State.ROUND
    assert len(coordinator.round.participant_ids) == 2


def test_correct_round_advertised_to_participants():
    """[summary]

    [extended_summary]
    """

    # test that only selected participants receive ROUND state and the others STANDBY
    coordinator = Coordinator(minimum_participants_in_round=1, fraction_of_participants=0.5)
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    # override selected participant
    coordinator.round.participant_ids = ["participant1"]

    # state ROUND will be advertised to participant1 (which has been selected)
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant1")
    assert result.state == coordinator_pb2.State.ROUND

    # state STANDBY will be advertised to participant2 (which has NOT been selected)
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant2")
    assert result.state == coordinator_pb2.State.STANDBY
