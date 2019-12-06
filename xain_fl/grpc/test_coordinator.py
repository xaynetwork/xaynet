from typing import List

import numpy as np
import pytest
from numproto import proto_to_ndarray
from numpy import ndarray

from xain_fl.grpc import coordinator_pb2
from xain_fl.grpc.coordinator import (
    Coordinator,
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)
from xain_fl.grpc.coordinator_pb2 import (
    RendezvousReply,
    RendezvousRequest,
    RendezvousResponse,
)


def test_rendezvous_accept():
    coordinator: Coordinator = Coordinator()
    result: RendezvousReply = coordinator.on_message(
        RendezvousRequest(), "participant1"
    )

    assert isinstance(result, RendezvousReply)
    assert result.response == RendezvousResponse.ACCEPT


def test_rendezvous_later_fraction_1():
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_rendezvous_later_fraction_05():
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=0.5,
    )

    # with 0.5 fraction it needs to accept at least two participants
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.ACCEPT

    # the third participant must receive LATER RendezvousResponse
    result = coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant3")

    assert isinstance(result, coordinator_pb2.RendezvousReply)
    assert result.response == coordinator_pb2.RendezvousResponse.LATER


def test_heartbeat_reply():
    # test that the coordinator replies with the correct state and round number
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant1")

    assert isinstance(result, coordinator_pb2.HeartbeatReply)
    assert result.state == coordinator_pb2.State.STANDBY
    assert result.round == 0

    # update the round of the coordinator and check again
    coordinator.current_round = 10
    result = coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant1")

    assert result.round == 10


def test_coordinator_state_standby_round():
    # tests that the coordinator transitions from STANDBY to ROUND once enough participants
    # are connected
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
    )

    assert coordinator.state == coordinator_pb2.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND
    assert coordinator.current_round == 1


def test_start_training():
    test_theta = [np.arange(10), np.arange(10, 20)]
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
        theta=test_theta,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    result = coordinator.on_message(
        coordinator_pb2.StartTrainingRequest(), "participant1"
    )
    received_weights = [proto_to_ndarray(nda) for nda in result.weights]

    np.testing.assert_equal(test_weights, received_weights)


def start_training_wrong_state():
    # if the coordinator receives a StartTraining request while not in the
    # ROUND state it will raise an exception
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    with pytest.raises(InvalidRequestError):
        coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "participant1")


def test_end_training():
    # we need two participants so that we can check the status of the local update mid round
    # with only one participant it wouldn't work because the local updates state is cleaned at
    # the end of each round
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    assert len(coordinator.round.updates) == 1


def test_end_training_round_update():
    # Test that the round number is updated once all participants sent their updates
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
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
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
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
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
        num_rounds=2,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    # Deliver results for 2 rounds
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.FINISHED


def test_wrong_participant():
    # coordinator should not accept requests from participants that it has not accepted
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.HeartbeatRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.StartTrainingRequest(), "participant2")

    with pytest.raises(UnknownParticipantError):
        coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant2")


def test_duplicated_update_submit():
    # the coordinator should not accept multiples updates from the same participant
    # in the same round
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")

    with pytest.raises(DuplicatedUpdateError):
        coordinator.on_message(coordinator_pb2.EndTrainingRequest(), "participant1")


def test_remove_participant():
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=1.0,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND

    coordinator.remove_participant("participant1")

    assert coordinator.participants.len() == 0
    assert coordinator.state == coordinator_pb2.State.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")

    assert coordinator.state == coordinator_pb2.State.ROUND


def test_number_of_selected_participants():
    # test that the coordinator needs minimum 3 participants and selects 2 of them
    coordinator = Coordinator(
        minimum_participants_in_round=2,
        fraction_of_participants=0.6,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    # the coordinator should not start a round but wait for a third
    assert coordinator.minimum_connected_participants == 3
    assert coordinator.state == coordinator_pb2.State.STANDBY

    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant3")

    # now the coordinator must have started a round and selected 2 participants
    assert coordinator.state == coordinator_pb2.State.ROUND
    assert len(coordinator.round.participant_ids) == 2



def test_correct_round_advertised_to_participants():
    # test that only selected participants receive ROUND state and the others STANDBY
    coordinator = Coordinator(
        minimum_participants_in_round=1,
        fraction_of_participants=0.5,
    )
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant1")
    coordinator.on_message(coordinator_pb2.RendezvousRequest(), "participant2")

    for participant_id in coordinator.participants.ids():
        result = coordinator.on_message(
            coordinator_pb2.HeartbeatRequest(), participant_id
        )

        # a participant selected in a round will receive ROUND state, else STANDBY
        if participant_id in coordinator.round.participant_ids:
            assert result.state == coordinator_pb2.State.ROUND
        else:
            assert result.state == coordinator_pb2.State.STANDBY
