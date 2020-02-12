"""XAIN FL tests for coordinator"""

from unittest import mock

import numpy as np
import pytest
from xain_proto.fl.coordinator_pb2 import (
    EndTrainingRoundRequest,
    HeartbeatRequest,
    RendezvousReply,
    RendezvousRequest,
    RendezvousResponse,
    StartTrainingRoundRequest,
    State,
)
from xain_proto.np import proto_to_ndarray

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.tools.exceptions import (
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)


def test_rendezvous_accept():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator: Coordinator = Coordinator()
    response: RendezvousResponse = coordinator.on_message(
        RendezvousRequest(), "participant1"
    )

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.ACCEPT


def test_rendezvous_later_fraction_1():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    response = coordinator.on_message(RendezvousRequest(), "participant2")

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.LATER


def test_rendezvous_later_fraction_05():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=0.5
    )

    # with 0.5 fraction it needs to accept at least two participants
    response = coordinator.on_message(RendezvousRequest(), "participant1")

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.ACCEPT

    response = coordinator.on_message(RendezvousRequest(), "participant2")

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.ACCEPT

    # the third participant must receive LATER RendezvousReply
    response = coordinator.on_message(RendezvousRequest(), "participant3")

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.LATER


def test_coordinator_state_standby_round():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # tests that the coordinator transitions from STANDBY to ROUND once enough participants
    # are connected
    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )

    assert coordinator.state == State.STANDBY

    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.state == State.ROUND
    assert coordinator.current_round == 0


def test_start_training_round():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    test_weights = np.arange(10)
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
        weights=test_weights,
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    response = coordinator.on_message(StartTrainingRoundRequest(), "participant1")
    received_weights = proto_to_ndarray(response.weights)

    np.testing.assert_equal(test_weights, received_weights)


def start_training_round_wrong_state():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # if the coordinator receives a StartTrainingRound request while not in the
    # ROUND state it will raise an exception
    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    with pytest.raises(InvalidRequestError):
        coordinator.on_message(StartTrainingRoundRequest(), "participant1")


@mock.patch("xain_fl.coordinator.store.NullObjectLocalWeightsReader.read_weights")
def test_end_training_round(read_weights_mock):
    """Test handling of a `EndTrainingRoundRequest` message.
    """

    # we need two participants so that we can check the status of the local update mid round
    # with only one participant it wouldn't work because the local updates state is cleaned at
    # the end of each round
    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    coordinator.on_message(EndTrainingRoundRequest(), "participant1")

    assert len(coordinator.round.updates) == 1
    read_weights_mock.assert_called_once_with("participant1", 0)


def test_end_training_round_update():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # Test that the round number is updated once all participants sent their updates
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
        num_rounds=2,
        epochs=3,
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    # check that we are currently in round 0
    assert coordinator.current_round == 0
    assert coordinator.epoch_base == 0

    coordinator.on_message(EndTrainingRoundRequest(), "participant1")
    # check we are still in round 0
    assert coordinator.current_round == 0
    assert coordinator.epoch_base == 0
    coordinator.on_message(EndTrainingRoundRequest(), "participant2")

    # check that round number was updated
    assert coordinator.current_round == 1
    assert coordinator.epoch_base == 3


def test_end_training_round_reinitialize_local_models():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    coordinator.on_message(EndTrainingRoundRequest(), "participant1")

    # After one participant sends its updates we should have one update in the coordinator
    assert len(coordinator.round.updates) == 1

    coordinator.on_message(EndTrainingRoundRequest(), "participant2")

    # once the second participant delivers its updates the round ends and the local models
    # are reinitialized
    assert coordinator.round.updates == {}


def test_training_finished():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    # Deliver results for 2 rounds
    coordinator.on_message(EndTrainingRoundRequest(), "participant1")
    coordinator.on_message(EndTrainingRoundRequest(), "participant1")

    assert coordinator.state == State.FINISHED


def test_wrong_participant():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # coordinator should not accept requests from participants that it has not accepted
    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(HeartbeatRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(StartTrainingRoundRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(EndTrainingRoundRequest(), "participant2")


def test_duplicated_update_submit():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # the coordinator should not accept multiples updates from the same participant
    # in the same round
    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    coordinator.on_message(EndTrainingRoundRequest(), "participant1")

    with pytest.raises(DuplicatedUpdateError):
        coordinator.on_message(EndTrainingRoundRequest(), "participant1")


def test_remove_participant():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.state == State.ROUND

    coordinator.remove_participant("participant1")

    assert coordinator.participants.len() == 0
    assert coordinator.state == State.STANDBY

    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.state == State.ROUND


def test_number_of_selected_participants():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # test that the coordinator needs minimum 3 participants and selects 2 of them
    coordinator = Coordinator(
        minimum_participants_in_round=2, fraction_of_participants=0.6
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    # the coordinator should wait for three participants to be connected before starting a round,
    # and select participants. Before that coordinator.round.participant_ids is an empty list
    assert coordinator.minimum_connected_participants == 3
    assert coordinator.state == State.STANDBY
    assert coordinator.round.participant_ids == []

    coordinator.on_message(RendezvousRequest(), "participant2")

    assert coordinator.state == State.STANDBY
    assert coordinator.round.participant_ids == []

    # add the third participant
    coordinator.on_message(RendezvousRequest(), "participant3")

    # now the coordinator must have started a round and selected 2 participants
    assert coordinator.state == State.ROUND
    assert len(coordinator.round.participant_ids) == 2


def test_correct_round_advertised_to_participants():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # test that only selected participants receive ROUND state and the others STANDBY
    coordinator = Coordinator(
        minimum_participants_in_round=1, fraction_of_participants=0.5
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    # override selected participant
    coordinator.round.participant_ids = ["participant1"]

    # state ROUND will be advertised to participant1 (which has been selected)
    response = coordinator.on_message(HeartbeatRequest(), "participant1")
    assert response.state == State.ROUND

    # state STANDBY will be advertised to participant2 (which has NOT been selected)
    response = coordinator.on_message(HeartbeatRequest(), "participant2")
    assert response.state == State.STANDBY
