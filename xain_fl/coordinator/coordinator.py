"""XAIN FL Coordinator"""

from typing import Dict, List, Optional, Tuple

from google.protobuf.internal.python_message import GeneratedProtocolMessageType
from numpy import ndarray
from xain_proto.fl.coordinator_pb2 import (
    EndTrainingRoundRequest,
    EndTrainingRoundResponse,
    HeartbeatRequest,
    HeartbeatResponse,
    RendezvousReply,
    RendezvousRequest,
    RendezvousResponse,
    StartTrainingRoundRequest,
    StartTrainingRoundResponse,
    State,
)
from xain_proto.numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.coordinator.participants import Participants
from xain_fl.coordinator.round import Round
from xain_fl.fl.coordinator.aggregate import Aggregator, FederatedAveragingAgg
from xain_fl.fl.coordinator.controller import Controller, RandomController
from xain_fl.logger import StructLogger, get_logger
from xain_fl.tools.exceptions import InvalidRequestError, UnknownParticipantError

logger: StructLogger = get_logger(__name__)


# TODO: raise exceptions for invalid attribute values: https://xainag.atlassian.net/browse/XP-387
class Coordinator:  # pylint: disable=too-many-instance-attributes
    """Class implementing the main Coordinator logic. It is implemented as a
    state machine that reacts to received messages.

    The states of the Coordinator are:

        - ``STANDBY``: The coordinator is in standby mode, typically
          when waiting for participants to connect. In this mode the
          only messages that the coordinator can receive are
          :class:`~.coordinator_pb2.RendezvousRequest` and
          :class:`~.coordinator_pb2.HeartbeatRequest`.

        - ``ROUND``: A round is currently in progress. During a round
          the important messages the coordinator can receive are
          :class:`~.coordinator_pb2.StartTrainingRoundRequest` and
          :class:`~.coordinator_pb2.EndTrainingRoundRequest`.  Since
          participants are selected for rounds or not, they can be
          advertised either ROUND or STANDBY accordingly.

        - ``FINISHED``: The training session has ended and
          participants should disconnect from the coordinator.

    States are exchanged during heartbeats so that both coordinators
    and participants can react to each others state change.

    The flow of the Coordinator:

        1. The coordinator is started and waits for enough participants to join. `STANDBY`.
        2. Once enough participants are connected the coordinator starts the rounds. `ROUND N`.
        3. Repeat step 2. for the given number of rounds
        4. The training session is over and the coordinator is ready to shutdown. `FINISHED`.

    Note:

        :class:`~.coordinator_pb2.RendezvousRequest` is always allowed
        regardless of which state the coordinator is on.

    Args:

        num_rounds: The number of rounds of the training session

        minimum_participants_in_round: The minimum number of
            participants that participate in a round

        fraction_of_participants: The fraction of total connected
            participants to be selected in a single round. Defaults to
            1.0, meaning that all connected participants will be
            selected. It must be in the (0.0, 1.0] interval.

        weights: The weights of the global model.

        epochs: Number of training iterations local to Participant.

        epochs_base: Global number of epochs as of last round.

        aggregator: The type of aggregation to perform at the end of
            each round. Defaults to :class:`~.FederatedAveragingAgg`.

        controller: Controls how the Participants are selected at the
            start of each round. Defaults to
            :class:`~.RandomController`.
    """

    DEFAULT_AGGREGATOR: Aggregator = FederatedAveragingAgg()
    """
    if no Aggregator instance is provided during initialisation, then
    :class:`~.FederatedAveragingAgg` in used.
    """

    DEFAULT_CONTROLLER: Controller = RandomController()
    """
    if no Controller instance is provided during initialisation, then
    :class:`~.RandomController` in used.
    """

    def __init__(  # pylint: disable=too-many-arguments,dangerous-default-value
        self,
        num_rounds: int = 1,
        minimum_participants_in_round: int = 1,
        fraction_of_participants: float = 1.0,
        weights: List[ndarray] = [],  # TODO: change to non-dangerous default value
        epochs: int = 1,
        epoch_base: int = 0,
        aggregator: Optional[Aggregator] = None,
        controller: Optional[Controller] = None,
    ) -> None:
        self.minimum_participants_in_round: int = minimum_participants_in_round
        self.fraction_of_participants: float = fraction_of_participants
        self.participants: Participants = Participants()
        self.num_rounds: int = num_rounds
        self.aggregator: Aggregator = aggregator if aggregator else self.DEFAULT_AGGREGATOR
        self.controller: Controller = controller if controller else self.DEFAULT_CONTROLLER
        self.minimum_connected_participants: int = self.get_minimum_connected_participants()

        # global model
        self.weights: List[ndarray] = weights
        self.epochs: int = epochs
        self.epoch_base: int = epoch_base

        # round updates
        self.round: Round = Round(self.participants.ids())

        # state variables
        self.state: State = State.STANDBY
        self.current_round: int = 0

    def get_minimum_connected_participants(self) -> int:
        """Calculates how many participants are needed so that we can select
        a specific fraction of them.
        Returns:
            obj:`int`: Minimum number of participants needed to be connected to start a round.
        """
        return int(self.minimum_participants_in_round // self.fraction_of_participants)

    def on_message(
        self, message: GeneratedProtocolMessageType, participant_id: str
    ) -> GeneratedProtocolMessageType:
        """Coordinator method that implements the state machine.

        Args:

            message: A protobuf message.
            participant_id: The id of the participant making the request.

        Returns:

            The response sent back to the participant.

        Raises:

            UnknownParticipantError: If it receives a request from an
                unknown participant. Typically a participant that has not
                rendezvous with the :class:`~.Coordinator`.

            InvalidRequestError: If it receives a request that is not
                allowed in the current :class:`~.Coordinator` state.
        """
        logger.debug(
            "Received message from participant",
            message_type=type(message),
            participant_id=participant_id,
        )

        # Unless this is a RendezvousRequest the coordinator should not accept messages
        # from participants that have not been accepted
        if (
            not isinstance(message, RendezvousRequest)
            and participant_id not in self.participants.ids()
        ):
            raise UnknownParticipantError(
                f"Unknown participant {participant_id}. "
                "Please try to rendezvous with the coordinator before making a request."
            )

        if isinstance(message, RendezvousRequest):
            # Handle rendezvous
            return self._handle_rendezvous(message, participant_id)

        if isinstance(message, HeartbeatRequest):
            # Handle heartbeat
            return self._handle_heartbeat(message, participant_id)

        if isinstance(message, StartTrainingRoundRequest):
            # handle start training
            return self._handle_start_training_round(message, participant_id)

        if isinstance(message, EndTrainingRoundRequest):
            # handle end training
            return self._handle_end_training_round(message, participant_id)

        raise NotImplementedError

    def remove_participant(self, participant_id: str) -> None:
        """Remove a participant from the list of accepted participants.

        This method is to be called when it is detected that a
        participant has disconnected. After a participant is removed,
        if the number of remaining participants is less than the
        minimum number of participants that need to be connected, the
        :class:`~.Coordinator` will transition to STANDBY state.

        Args:

            participant_id: The id of the participant to remove.

        """
        self.participants.remove(participant_id)
        logger.info("Removing participant", participant_id=participant_id)

        if self.participants.len() < self.minimum_connected_participants:
            self.state = State.STANDBY

    def select_participant_ids_and_init_round(self) -> None:
        """Selects the participant ids and initiates a Round.
        """
        self.controller.fraction_of_participants = self.fraction_of_participants
        selected_ids = self.controller.select_ids(self.participants.ids())
        self.round = Round(selected_ids)

    def _handle_rendezvous(
        self, _message: RendezvousRequest, participant_id: str
    ) -> RendezvousResponse:
        """Handles a Rendezvous request.

        Args:

            _message: The request to handle. Currently not used.
            participant_id: The id of the participant making the request.

        Returns:

            The response to the participant.
        """

        if self.participants.len() < self.minimum_connected_participants:
            reply = RendezvousReply.ACCEPT
            self.participants.add(participant_id)
            logger.info(
                "Accepted participant",
                participant_id=participant_id,
                current_participants_count=self.participants.len(),
            )

            # Select participants and change the state to ROUND if the latest added participant
            # lets us meet the minimum number of connected participants
            if self.participants.len() == self.minimum_connected_participants:
                self.select_participant_ids_and_init_round()

                # TODO: We may need to make this update thread safe
                self.state = State.ROUND
                self.current_round = 1 if self.current_round == 0 else self.current_round
        else:
            reply = RendezvousReply.LATER
            logger.info(
                "Reject participant",
                participant_id=participant_id,
                current_participants_count=self.participants.len(),
            )

        return RendezvousResponse(reply=reply)

    def _handle_heartbeat(
        self, _message: HeartbeatRequest, participant_id: str
    ) -> HeartbeatResponse:
        """Handles a Heartbeat request.

        Responds to the participant with:

            - ``FINISHED``: if coordinator is in state FINISHED,
            - ``ROUND``: if the participant is selected for the current round,
            - ``STANDBY``: if the participant is not selected for the current round.

        Args:

            _message: The request to handle. Currently not used.
            participant_id: The id of the participant making the request.

        Returns:

            The response to the participant.
        """
        self.participants.update_expires(participant_id)

        if self.state == State.FINISHED or participant_id in self.round.participant_ids:
            state = self.state
        else:
            state = State.STANDBY

        # send heartbeat response advertising the current state
        logger.debug(
            "Heartbeat response",
            participant_id=participant_id,
            message=state,
            round=self.current_round,
        )
        return HeartbeatResponse(state=state, round=self.current_round)

    def _handle_start_training_round(
        self, _message: StartTrainingRoundRequest, participant_id: str
    ) -> StartTrainingRoundResponse:
        """Handles a StartTrainingRound request.

        Args:
            _message: The request to handle. Currently not used.
            participant_id: The id of the participant making the request.
        Returns:
            :class:`~.coordinator_pb2.StartTrainingRoundResponse`: The response to the participant.
        """
        # The coordinator should only accept StartTrainingRound requests if it is
        # in the ROUND state and when the participant has been selected for the round.
        coordinator_not_in_a_round = self.state != State.ROUND
        participant_not_selected = participant_id not in self.round.participant_ids
        if coordinator_not_in_a_round or participant_not_selected:
            raise InvalidRequestError(
                f"Participant {participant_id} sent a "
                "StartTrainingRoundRequest outside of a round"
            )

        weights_proto = [ndarray_to_proto(nda) for nda in self.weights]

        return StartTrainingRoundResponse(
            weights=weights_proto, epochs=self.epochs, epoch_base=self.epoch_base,
        )

    def _handle_end_training_round(
        self, message: EndTrainingRoundRequest, participant_id: str
    ) -> EndTrainingRoundResponse:
        """Handles a EndTrainingRound request.

        Args:

            message: The request to handle.
            participant_id: The id of the participant making the request.

        Returns:

            The response to the participant.
        """

        # TODO: Ideally we want to know for which round the participant is
        # submitting the updates and raise an exception if it is the wrong
        # round.

        # record the request data
        weight_update: Tuple[List[ndarray], int] = (
            [proto_to_ndarray(pnda) for pnda in message.weights],
            message.number_samples,
        )
        metrics: Dict[str, ndarray] = {k: proto_to_ndarray(v) for k, v in message.metrics.items()}
        self.round.add_updates(participant_id, weight_update, metrics)

        # The round is over. Run the aggregation
        if self.round.is_finished():
            logger.info("Running aggregation for round", current_round=self.current_round)

            self.weights = self.aggregator.aggregate(self.round.get_weight_updates())

            # update the round or finish the training session
            if self.current_round == self.num_rounds:
                logger.debug("Last round over", round=self.current_round)
                self.state = State.FINISHED
            else:
                self.current_round += 1
                # reinitialize the round
                self.select_participant_ids_and_init_round()

        return EndTrainingRoundResponse()
