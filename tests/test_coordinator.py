"""XAIN FL tests for coordinator"""

import numpy as np
from numpy import ndarray
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

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.fl.coordinator.controller import OrderController
from xain_fl.tools.exceptions import (
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)

# pylint: disable=redefined-outer-name


def test_rendezvous_accept(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator: Coordinator = coordinator()
    response: RendezvousResponse = coordinator.on_message(
        RendezvousRequest(), "participant1"
    )

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.ACCEPT


def test_rendezvous_later_fraction_1(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    response = coordinator.on_message(RendezvousRequest(), "participant2")

    assert isinstance(response, RendezvousResponse)
    assert response.reply == RendezvousReply.LATER


def test_rendezvous_later_fraction_05(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = coordinator(
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


def test_coordinator_state_standby_round(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # tests that the coordinator transitions from STANDBY to ROUND once enough participants
    # are connected
    coordinator = coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )

    assert coordinator.state == State.STANDBY

    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.state == State.ROUND
    assert coordinator.current_round == 0


def start_training_round_wrong_state(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # if the coordinator receives a StartTrainingRound request while not in the
    # ROUND state it will raise an exception
    coordinator = coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    with pytest.raises(InvalidRequestError):
        coordinator.on_message(StartTrainingRoundRequest(), "participant1")


def test_end_training_round(coordinator, end_training_request):
    """Test handling of a `EndTrainingRoundRequest` message.
    """

    # we need two participants so that we can check the status of the local update mid round
    # with only one participant it wouldn't work because the local updates state is cleaned at
    # the end of each round
    coordinator = coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    local_weights = ndarray([1, 2, 3])
    end_training_request(coordinator, "participant1", 0, local_weights)

    assert len(coordinator.round.updates) == 1
    # check that the coordinator read the correct local weights from the store
    coordinator.local_weights_reader.assert_read("participant1", 0)
    update = coordinator.round.updates["participant1"]
    np.testing.assert_array_equal(update["model_weights"], local_weights)


def test_end_training_round_update(coordinator, end_training_request):
    """Test that the round number is updated once all participants sent
    their updates

    """

    coordinator = coordinator(
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

    # Pretend participant1 sent their result to s3 and then sent an
    # EndTraining request
    end_training_request(coordinator, "participant1")

    # check we are still in round 0
    assert coordinator.current_round == 0
    assert coordinator.epoch_base == 0

    # Pretend participant2 sent their result to s3, and then sent an
    # EndTraining request
    end_training_request(coordinator, "participant2")

    # check that round number was updated
    assert coordinator.current_round == 1
    assert coordinator.epoch_base == 3


def test_end_training_round_reinitialize_local_models(
    coordinator, end_training_request
):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    end_training_request(coordinator, "participant1")

    # After one participant sends its updates we should have one update in the coordinator
    assert len(coordinator.round.updates) == 1

    end_training_request(coordinator, "participant2")

    # once the second participant delivers its updates the round ends and the local models
    # are reinitialized
    assert coordinator.round.updates == {}


def test_training_finished(coordinator, end_training_request):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0, num_rounds=2
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    # Deliver results for 2 rounds
    end_training_request(coordinator, "participant1", round=0)
    end_training_request(coordinator, "participant1", round=1)

    assert coordinator.state == State.FINISHED


def test_wrong_participant(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # coordinator should not accept requests from participants that it has not accepted
    coordinator = coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(HeartbeatRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(StartTrainingRoundRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(EndTrainingRoundRequest(), "participant2")


def test_duplicated_update_submit(coordinator, end_training_request):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # the coordinator should not accept multiples updates from the same participant
    # in the same round
    coordinator = coordinator(
        minimum_participants_in_round=2, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")

    end_training_request(coordinator, "participant1")

    with pytest.raises(DuplicatedUpdateError):
        end_training_request(coordinator, "participant1")


def test_remove_selected_participant(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    coordinator = coordinator(
        minimum_participants_in_round=1, fraction_of_participants=1.0
    )
    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.participants.len() == 1
    assert coordinator.round.participant_ids == ["participant1"]
    assert coordinator.state == State.ROUND

    coordinator.remove_participant("participant1")

    assert coordinator.participants.len() == 0
    assert coordinator.round.participant_ids == []
    assert coordinator.state == State.STANDBY

    coordinator.on_message(RendezvousRequest(), "participant1")

    assert coordinator.participants.len() == 1
    assert coordinator.round.participant_ids == ["participant1"]
    assert coordinator.state == State.ROUND


def test_number_of_selected_participants(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # test that the coordinator needs minimum 3 participants and selects 2 of them
    coordinator = coordinator(
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


def test_correct_round_advertised_to_participants(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # test that only selected participants receive ROUND state and the others STANDBY
    coordinator = coordinator(
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


def test_select_outstanding(coordinator):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    # setup: select first 3 of 4 in order per round
    coordinator = coordinator(
        minimum_participants_in_round=3,
        fraction_of_participants=0.75,
        controller=OrderController(),
    )
    coordinator.on_message(RendezvousRequest(), "participant1")
    coordinator.on_message(RendezvousRequest(), "participant2")
    coordinator.on_message(RendezvousRequest(), "participant3")
    coordinator.on_message(RendezvousRequest(), "participant4")

    # 4 connected hence round starts
    assert coordinator.state == State.ROUND
    assert coordinator.participants.len() == 4
    # selection is triggered: order-controller guarantees it's [P1, P2, P3]
    assert coordinator.round.participant_ids == [
        "participant1",
        "participant2",
        "participant3",
    ]

    coordinator.remove_participant("participant3")

    # round pauses
    assert coordinator.state == State.STANDBY
    assert coordinator.participants.len() == 3
    assert coordinator.round.participant_ids == ["participant1", "participant2"]

    coordinator.remove_participant("participant1")

    assert coordinator.participants.len() == 2
    assert coordinator.round.participant_ids == ["participant2"]

    coordinator.on_message(RendezvousRequest(), "participant5")
    coordinator.on_message(RendezvousRequest(), "participant6")

    # back up to 4 (P2, P4, P5, P6) so round resumes
    assert coordinator.state == State.ROUND
    assert coordinator.participants.len() == 4
    # selection triggered: P2 still selected with 2 outstanding from [P4, P5, P6]
    assert coordinator.round.participant_ids == [
        "participant2",
        "participant4",
        "participant5",
    ]
