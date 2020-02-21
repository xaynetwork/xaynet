"""XAIN FL Coordinator"""

from typing import Dict, List, Optional, Union

from google.protobuf.descriptor import EnumDescriptor
from google.protobuf.internal.python_message import GeneratedProtocolMessageType
import numpy as np
from numpy import ndarray
from structlog import get_logger
from xain_proto.fl.coordinator_pb2 import (
    _RENDEZVOUSREPLY,
    _STATE,
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

from xain_fl.coordinator.metrics_store import (
    AbstractMetricsStore,
    MetricsStoreError,
    NullObjectMetricsStore,
)
from xain_fl.coordinator.participants import Participants
from xain_fl.coordinator.round import Round
from xain_fl.coordinator.session import Session
from xain_fl.coordinator.store import (
    AbstractGlobalWeightsWriter,
    AbstractLocalWeightsReader,
)
from xain_fl.fl.coordinator.aggregate import Aggregator, WeightedAverageAggregator
from xain_fl.fl.coordinator.controller import Controller, RandomController
from xain_fl.logger import StructLogger
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
          participants may or may not be selected for rounds, they can be
          advertised accordingly with ROUND or STANDBY respectively.
          Round numbers start from 0.

        - ``FINISHED``: The training session has ended and
          participants should disconnect from the coordinator.

    States are exchanged during heartbeats so that both coordinators
    and participants can react to each others state change.

    The flow of the Coordinator:
        1. The coordinator is started and waits for enough participants to join. `STANDBY`.
        2. Once enough participants are connected the coordinator starts the rounds. `ROUND`.
        3. Repeat step 2. for the given number of rounds
        4. The training session is over and the coordinator is ready to shutdown. `FINISHED`.

    Note:
        :class:`~.coordinator_pb2.RendezvousRequest` is always allowed
        regardless of which state the coordinator is on.

    Args:

        global_weights_writer: service for storing global weights

        local_weights_reader: service for retrieving the local weights

        num_rounds: The number of rounds of the training session.

        minimum_participants_in_round: The minimum number of
            participants that participate in a round.

        fraction_of_participants: The fraction of total connected
            participants to be selected in a single round. Defaults to
            1.0, meaning that all connected participants will be
            selected. It must be in the (0.0, 1.0] interval.

        weights: The weights of the global model.

        epochs: Number of training iterations local to Participant.

        epochs_base: The global epoch number for the start of the next training round.

        aggregator: The type of aggregation to perform at the end of
            each round. Defaults to :class:`~.WeightedAverageAggregator`.

        controller: Controls how the Participants are selected at the
            start of each round. Defaults to :class:`~.RandomController`.

    """

    def __init__(  # pylint: disable=too-many-arguments
        self,
        global_weights_writer: AbstractGlobalWeightsWriter,
        local_weights_reader: AbstractLocalWeightsReader,
        metrics_store: AbstractMetricsStore = NullObjectMetricsStore(),
        num_rounds: int = 1,
        minimum_participants_in_round: int = 1,
        fraction_of_participants: float = 1.0,
        weights: ndarray = np.empty(shape=(0,)),
        epochs: int = 1,
        epoch_base: int = 0,
        aggregator: Aggregator = WeightedAverageAggregator(),
        controller: Controller = RandomController(),
    ) -> None:
        self.global_weights_writer: AbstractGlobalWeightsWriter = global_weights_writer
        # pylint: disable=line-too-long
        self.local_weights_reader: AbstractLocalWeightsReader = local_weights_reader
        self.minimum_participants_in_round: int = minimum_participants_in_round
        self.fraction_of_participants: float = fraction_of_participants
        self.participants: Participants = Participants()
        self.num_rounds: int = num_rounds
        self.aggregator: Aggregator = aggregator
        self.controller: Controller = controller
        self.metrics_store = metrics_store
        self.minimum_connected_participants: int = self.get_minimum_connected_participants()

        # global model
        # self.weights: ndarray = weights
        self.epochs: int = epochs
        # self.epoch_base: int = epoch_base

        # round updates
        self.round: Round = Round(self.participants.ids())

        # state variables
        # self.state: State = State.STANDBY
        # self.current_round: int = 0
        self.epochs_current_round: int = epochs

        # session state
        self.session: Session = Session(State.STANDBY, 0, epoch_base, weights)

        # Write the weights for the initial round
        self.global_weights_writer.write_weights(0, weights)

        self._write_metrics_fail_silently(
            "coordinator",
            {
                "state": State.STANDBY,
                "round": 0,
                "number_of_selected_participants": 0,
            },
        )

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
            message_type=message.DESCRIPTOR.name,
            message_byte_size=message.ByteSize(),
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

        logger.info("Removing participant", participant_id=participant_id)
        self.participants.remove(participant_id)
        # remove from selected if necessary
        self.round.remove_selected(participant_id)

        parts_len = self.participants.len()
        if parts_len < self.minimum_connected_participants:
            self.session.set_state(State.STANDBY)
            self._write_metrics_fail_silently("coordinator", {"state": State.STANDBY})

        self._write_metrics_fail_silently(
            "participant", {"state": State.FINISHED}, tags={"id": participant_id}
        )
        self._write_metrics_fail_silently(
            "coordinator", {"number_of_selected_participants": parts_len}
        )

    def select_participant_ids_and_init_round(self) -> None:
        """Selects the participant ids and initiates a Round."""

        self.controller.fraction_of_participants = self.fraction_of_participants
        selected_ids = self.controller.select_ids(self.participants.ids())
        self.round = Round(selected_ids)

    def select_outstanding(self) -> List[str]:
        """Selects participants outstanding for the round."""

        selected = set(self.round.participant_ids)

        # the following preconditions should hold
        assert len(selected) < self.minimum_participants_in_round
        assert self.participants.len() == self.minimum_connected_participants

        num_outstanding = self.minimum_participants_in_round - len(selected)
        pool = set(self.participants.ids()) - selected
        frac = num_outstanding / len(pool)

        self.controller.fraction_of_participants = frac
        return self.controller.select_ids(list(pool))

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

        parts_len = self.participants.len()
        if parts_len < self.minimum_connected_participants:
            reply = RendezvousReply.ACCEPT
            self.participants.add(participant_id)
            logger.info(
                "Accepted participant",
                participant_id=participant_id,
                current_participants_count=parts_len,
            )
            self._write_metrics_fail_silently(
                "coordinator",
                {"number_of_selected_participants": parts_len},
            )

            # Select participants and change the state to ROUND if the latest added participant
            # lets us meet the minimum number of connected participants
            if parts_len == self.minimum_connected_participants:
                # select enough to fill round if needed
                if len(self.round.participant_ids) < self.minimum_participants_in_round:
                    ids = self.select_outstanding()
                    self.round.add_selected(ids)

                self.session.set_state(State.ROUND)
                self._write_metrics_fail_silently("coordinator", {"state": State.ROUND})
        else:
            reply = RendezvousReply.LATER
            logger.info(
                "Reject participant",
                participant_id=participant_id,
                current_participants_count=parts_len,
            )

        logger.debug(
            "Send RendezvousResponse", reply=pb_enum_to_str(_RENDEZVOUSREPLY, reply)
        )
        return RendezvousResponse(reply=reply)

    def _handle_heartbeat(
        self, message: HeartbeatRequest, participant_id: str
    ) -> HeartbeatResponse:
        """Handles a Heartbeat request.

        Responds to the participant with:
            - ``FINISHED``: if coordinator is in state FINISHED,
            - ``ROUND``: if the participant is selected for the current round,
            - ``STANDBY``: if the participant is not selected for the current round.

        Args:
            message: The request to handle. Currently not used.
            participant_id: The id of the participant making the request.

        Returns:
            The response to the participant.
        """

        self._write_metrics_fail_silently(
            "participant",
            {"state": message.state, "round": message.round},
            tags={"id": participant_id},
        )

        self.participants.update_expires(participant_id)

        current_state = self.session.get_state()
        if current_state == State.FINISHED or participant_id in self.round.participant_ids:
            state = current_state
        else:
            state = State.STANDBY

        current_round = self.session.get_round()
        logger.debug(
            "Heartbeat response",
            participant_id=participant_id,
            state=pb_enum_to_str(_STATE, state),
            round=current_round,
            current_participants_count=self.participants.len(),
        )
        self._write_metrics_fail_silently(
            "coordinator", {"number_of_selected_participants": self.participants.len()}
        )
        # send heartbeat response advertising the current state
        return HeartbeatResponse(state=state, round=current_round)

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
        coordinator_not_in_a_round = self.session.get_state() != State.ROUND
        participant_not_selected = participant_id not in self.round.participant_ids
        if coordinator_not_in_a_round or participant_not_selected:
            raise InvalidRequestError(
                f"Participant {participant_id} sent a "
                "StartTrainingRoundRequest outside of a round"
            )

        weights = self.session.get_weights()
        if weights.size:
            self.epochs_current_round = self.epochs
        else:
            self.epochs_current_round = 0

        epoch_base = self.session.get_epoch_base()
        logger.debug(
            "Send StartTrainingRoundResponse",
            epochs=self.epochs_current_round,
            epoch_base=epoch_base,
        )
        return StartTrainingRoundResponse(
            epochs=self.epochs_current_round, epoch_base=epoch_base,
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

        # HACK: Participants are currently not aware of the ID the
        # coordinator uses to identify them, so they generate a UUID
        # and use it to upload their results. The EndTrainingMessage
        # contains that UUID, so that the coordinator can retrieve the
        # weights.
        #
        # FIXME(PB-436): handle the case where the participant didn't
        # send a participant ID, or sent an invalid one. Reading from
        # storage will fail in that case, and we should gracefully
        # handle this by removing the participant.
        current_round = self.session.get_round()
        fake_participant_id: str = message.participant_id
        logger.debug("downloading results", participant_id=participant_id)
        model_weights: ndarray = self.local_weights_reader.read_weights(
            fake_participant_id, current_round
        )
        logger.debug("done downloading results", participant_id=participant_id)
        number_samples: int = message.number_samples
        self.round.add_updates(
            participant_id=participant_id,
            model_weights=model_weights,
            aggregation_data=number_samples,
        )

        try:
            if message.metrics != "[]":
                self.metrics_store.write_received_participant_metrics(message.metrics)
        except MetricsStoreError as err:
            logger.warn(
                "Can not write metrics", participant_id=participant_id, error=repr(err)
            )

        # The round is over. Run the aggregation
        if self.round.is_finished():
            logger.info(
                "Running aggregation for round", current_round=current_round
            )

            multiple_model_weights: List[ndarray]
            aggregation_data: List[int]
            multiple_model_weights, aggregation_data = self.round.get_weight_updates()
            weights = self.aggregator.aggregate(
                multiple_model_weights=multiple_model_weights,
                aggregation_data=aggregation_data,
            )
            self.session.set_weights(weights)
            self.global_weights_writer.write_weights(
                current_round + 1, weights
            )

            # update the round or finish the training session
            if current_round >= self.num_rounds - 1:
                logger.info("Last round over", round=current_round)
                self.session.set_state(State.FINISHED)
                self._write_metrics_fail_silently("coordinator", {"state": State.FINISHED})
            else:
                self.session.next_round()
                self.session.add_epochs(self.epochs_current_round)
                self._write_metrics_fail_silently(
                    "coordinator", {"round": current_round + 1}
                )
                # reinitialize the round
                self.select_participant_ids_and_init_round()

        logger.debug("Send EndTrainingRoundResponse", participant_id=participant_id)
        return EndTrainingRoundResponse()

    def _write_metrics_fail_silently(
        self,
        owner: str,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to a metric store that are collected on the coordinator site and owned by
        the given owner.
        If an exception is raised, it will be caught and the error logged.

        FIXME: Helper function to make sure that the coordinator does not crash due to exception of
        the metric store. Proper exception handling should be tackled in PB-125.

        Args:

            owner: The name of the owner of the metrics e.g. coordinator or participant.
            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.
        """

        try:
            self.metrics_store.write_metrics(owner, metrics, tags)
        except MetricsStoreError as err:
            logger.warn("Can not write metrics", error=repr(err), owner=owner)


def pb_enum_to_str(pb_enum: EnumDescriptor, member_value: int) -> str:
    """Return the human readable string of a enum member value.

    Args:
        pb_enum: The proto enum definition.
        member_value:  The enum member value.

    Returns:
        The human readable string of a enum member value.
    """
    enum_in_str: str = pb_enum.values_by_number[member_value].name
    return enum_in_str
