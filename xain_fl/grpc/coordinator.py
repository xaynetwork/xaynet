"""Module implementing the networked coordinator using gRPC.

This module implements the Coordinator state machine, the Coordinator gRPC
service and helper class to keep state about the Participants.
"""
import threading
import time
from concurrent import futures
from typing import Dict, List, Tuple

import grpc
from google.protobuf.internal.python_message import GeneratedProtocolMessageType
from numproto import ndarray_to_proto, proto_to_ndarray
from numpy import ndarray

from xain_fl.fl.coordinator.aggregate import FederatedAveragingAgg
from xain_fl.fl.coordinator.controller import RandomController
from xain_fl.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain_fl.logger import get_logger

logger = get_logger(__name__)


_ONE_DAY_IN_SECONDS: int = 60 * 60 * 24
HEARTBEAT_TIME: int = 10
HEARTBEAT_TIMEOUT: int = 5


class DuplicatedUpdateError(Exception):
    """Exception raised when the same participant tries to submit multiple
    updates to the :class:`~.Coordinator` in the same :class:`~.Round`
    """


class UnknownParticipantError(Exception):
    """Exception raised when a participant that is unknown to the
    :class:`~.Coordinator` makes a request.

    Typically this means that a participant tries to make a request before it
    has successfully rendezvous with the :class:`~.Coordinator`.
    """


class InvalidRequestError(Exception):
    """Exception raised when the Coordinator receives and invalid request from a participant.

    This can happen if the participant sends a request that is not allowed in a
    give Coordinator state. For instance the Coordinator will only accept
    StartTraining requests during a ROUND.
    """


class ParticipantContext:
    """Class to store state about each participant. Currently it only stores the `participant_id`
    and the time when the next heartbeat_expires.

    In the future we may store more information like in what state a participant is in e.g.
    IDLE, RUNNING, ...

    Args:
        participant_id (:obj:`str`): The id of the participant. Typically a
            host:port or public key when using SSL.
    """

    def __init__(self, participant_id: str) -> None:
        self.participant_id: str = participant_id
        self.heartbeat_expires: float = time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT


class Participants:
    """This class provides some useful methods to handle all the participants
    connected to a coordinator in a thread safe manner by protecting access to
    the participants list with a lock.
    """

    def __init__(self) -> None:
        self.participants: Dict[str, ParticipantContext] = {}
        self._lock: Lock = threading.Lock()

    def add(self, participant_id: str) -> None:
        """Adds a new participant to the list of participants.

        Args:
            participant_id (:obj:`str`): The id of the participant to add.
        """

        with self._lock:
            self.participants[participant_id] = ParticipantContext(participant_id)

    def remove(self, participant_id: str) -> None:
        """Removes a participant from the list of participants.

        This will be typically used after a participant is disconnected from the coordinator.

        Args:
            participant_id (:obj:`str`): The id of the participant to remove.
        """

        with self._lock:
            if participant_id in self.participants:
                del self.participants[participant_id]

    def next_expiration(self) -> float:
        """Helper method to check what is the next heartbeat to expire.

        Currently being used by the `heartbeat_monitor` to check how long it should sleep until
        the next check.

        Returns:
            :obj:`float`: The next heartbeat to expire.
        """

        with self._lock:
            if self.participants:
                return min([p.heartbeat_expires for p in self.participants.values()])

        return time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT

    def len(self) -> int:
        """Get the number of participants.

        Returns:
            :obj:`int`: The number of participants in the list.
        """

        with self._lock:
            return len(self.participants)

    def ids(self) -> List[str]:
        """Get the ids of the participants.

        Returns:
            :obj:`list` of :obj:`str`: The list of participant ids.
        """

        with self._lock:
            return list(self.participants.keys())

    def update_expires(self, participant_id: str) -> None:
        """Updates the heartbeat expiration time for a participant.

        This is currently called by the :class:`~.Coordinator` every time a participant sends a
        heartbeat.

        Args:
            participant_id (:obj:`str`): The id of the participant to update the expire time.
        """

        with self._lock:
            self.participants[participant_id].heartbeat_expires = (
                time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT
            )


class Round:
    """Class to manage the state of a single round.

    This class contains the logic to handle all updates sent by the
    participants during a round and does some sanity checks like preventing the
    same participant to submit multiple updates during a single round.

    Args:
        participant_ids(:obj:`list` of :obj:`str`): The list of IDs of the participants
            selected to participate in this round.
    """

    def __init__(self, participant_ids: List[str]) -> None:
        self.participant_ids = participant_ids
        self.updates: Dict[str, Dict] = {}

    def add_updates(
        self,
        participant_id: str,
        weight_update: Tuple[List[ndarray], int],
        metrics: Dict[str, List[ndarray]],
    ) -> None:
        """Valid a participant's update for the round.

        Args:
            participant_id (:obj:`str`): The id of the participant making the request.
            weight_update (:obj:`tuple` of :obj:`list` of :class:`~numpy.ndarray`):
                A tuple containing a list of updated weights.
            metrics (:obj:`dict`): A dictionary containing metrics with the name and the value
                as list of ndarrays.

        Raises:
            DuplicatedUpdateError: If the participant already submitted his update this round.
        """

        if participant_id in self.updates.keys():
            raise DuplicatedUpdateError(
                f"Participant {participant_id} already submitted the update for this round."
            )

        self.updates[participant_id] = {
            "weight_update": weight_update,
            "metrics": metrics,
        }

    def is_finished(self) -> bool:
        """Check if all the required participants submitted their updates this round.

        If all participants submitted their updates the round is considered finished.

        Returns:
            :obj:`bool`:: :obj:`True` if all participants submitted their
            updates this round. :obj:`False` otherwise.
        """
        return len(self.updates) == len(self.participant_ids)

    def get_weight_updates(self) -> List[Tuple[List[ndarray], int]]:
        """Get a list of all participants weight updates.

        This list will usually be used by the aggregation function.

        Returns:
            :obj:`list` of :obj:`tuple`: The list of weight updates from all
            participants.
        """
        return [v["weight_update"] for k, v in self.updates.items()]


class Coordinator:
    """Class implementing the main Coordinator logic. It is implemented as a
    state machine that reacts to received messages.

    The states of the Coordinator are:
        STANDBY: The coordinator is in standby mode, typically when waiting for
        participants to connect. In this mode the only messages that the
        coordinator can receive are :class:`~.coordinator_pb2.RendezvousRequest`
        and :class:`~.coordinator_pb2.HeartbeatRequest`.

        ROUND: A round is currently in progress. During a round the important
        messages the coordinator can receive are
        :class:`~.coordinator_pb2.StartTrainingRequest` and
        :class:`~.coordinator_pb2.EndTrainingRequest`.
        Since participants are selected for rounds or not, they can be advertised
        either ROUND or STANDBY accordingly.

        FINISHED: The training session has ended and participants should
        disconnect from the coordinator.

    States are exchanged during heartbeats so that both coordinators and
    participants can react to each others state change.

    The flow of the Coordinator:
        1. The coordinator is started and waits for enough participants to join. `STANDBY`.
        2. Once enough participants are connected the coordinator starts the rounds. `ROUND N`.
        3. Repeat step 2. for the given number of rounds
        4. The training session is over and the coordinator is ready to shutdown. `FINISHED`.

    Note:
        :class:`~.coordinator_pb2.RendezvousRequest` is always allowed
        regardless of which state the coordinator is on.

    Args:
        num_rounds (:obj:`int`, optional): The number of rounds of the training
            session. Defaults to 10.
        minimum_participants_in_round (:obj:`float`, optional): The minimum number of
            participants that participate in a round. Defaults to 1.
        fraction_of_participants (:obj:`float`, optional): The fraction of total
            connected participants to be selected in a single round. Defaults to 1.0,
            meaning that all connected participants will be selected.
        theta (:obj:`list` of :class:`~numpy.ndarray`, optional): The weights of
            the global model. Defaults to [].
        epochs (:obj:`int`, optional): Number of training iterations local to
            Participant.  Defaults to 0.
        epochs_base (:obj:`int`, optional): Global number of epochs as of last
            round. Defaults to 0.
        """

    # pylint: disable-msg=too-many-instance-attributes
    def __init__(
        self,
        num_rounds: int = 1,
        minimum_participants_in_round: int = 1,
        fraction_of_participants: float = 1.0,
        theta: List[np.ndarray] = [],
        epochs: int = 1,
        epoch_base: int = 0,
    ) -> None:
        self.minimum_participants_in_round = minimum_participants_in_round
        self.fraction_of_participants = fraction_of_participants
        self.participants = Participants()
        self.num_rounds = num_rounds
        self.aggregator = FederatedAveragingAgg()
        self.controller = RandomController(self.participants.ids())
        self.minimum_connected_participants = self.get_minimum_connected_participants()

        # global model
        self.weights: List[ndarray] = weights
        self.epochs: int = epochs
        self.epoch_base: int = epoch_base

        # round updates
        self.round = Round(self.participants.ids())

        # state variables
        self.state = coordinator_pb2.State.STANDBY
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
            message (:class:`~.GeneratedProtocolMessageType`): A protobuf message.
            participant_id (:obj:`str`): The id of the participant making the request.

        Returns:
            :class:`~.GeneratedProtocolMessageType`: The reply to be sent back to the participant.

        Raises:
            :class:`~UnknownParticipantError`: If it receives a request from an
            unknown participant. Typically a participant that has not
            rendezvous with the :class:`~.Coordinator`.
            :class:`~InvalidRequestError`: If it receives a request that is not
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
            not isinstance(message, coordinator_pb2.RendezvousRequest)
            and participant_id not in self.participants.ids()
        ):
            raise UnknownParticipantError(
                f"Unknown participant {participant_id}. "
                "Please try to rendezvous with the coordinator before making a request."
            )

        # pylint: disable-msg=no-else-return
        if isinstance(message, coordinator_pb2.RendezvousRequest):
            # Handle rendezvous
            return self._handle_rendezvous(message, participant_id)

        elif isinstance(message, coordinator_pb2.HeartbeatRequest):
            # Handle heartbeat
            return self._handle_heartbeat(message, participant_id)

        elif isinstance(message, coordinator_pb2.StartTrainingRequest):
            # handle start training
            return self._handle_start_training(message, participant_id)

        elif isinstance(message, coordinator_pb2.EndTrainingRequest):
            # handle end training
            return self._handle_end_training(message, participant_id)

        else:
            raise NotImplementedError

    def remove_participant(self, participant_id: str) -> None:
        """Remove a participant from the list of accepted participants.

        This method is to be called when it is detected that a participant has
        disconnected.

        After a participant is removed, if the number of remaining participants
        is less than the minimum number of participants that need to be connected,
        the :class:`~.Coordinator` will transition to STANDBY state.

        Args:
            participant_id (:obj:`str`): The id of the participant to remove.
        """
        self.participants.remove(participant_id)
        logger.info("Removing participant", participant_id=participant_id)

        if self.participants.len() < self.minimum_connected_participants:
            self.state = coordinator_pb2.State.STANDBY

    def select_participant_ids_and_init_round(self) -> None:
        """Initiates the Controller, selects ids and initiates a Round.
        """
        self.controller = RandomController(
            participants_ids=self.participants.ids(),
            fraction_of_participants=self.fraction_of_participants,
        )
        selected_ids = self.controller.select_ids()
        self.round = Round(selected_ids)

    def _handle_rendezvous(
        self, _message: coordinator_pb2.RendezvousRequest, participant_id: str
    ) -> coordinator_pb2.RendezvousReply:
        """Handles a Rendezvous request.

        Args:
            _message (:class:`~.coordinator_pb2.RendezvousRequest`): The
                request to handle. Currently not used.
            participant_id (:obj:`str`): The id of the participant making the
                request.

        Returns:
            :class:`~.coordinator_pb2.RendezvousReply`: The reply to the participant.
        """

        if self.participants.len() < self.minimum_connected_participants:
            response = coordinator_pb2.RendezvousResponse.ACCEPT
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
                self.state = coordinator_pb2.State.ROUND
                self.current_round = (
                    1 if self.current_round == 0 else self.current_round
                )
        else:
            response = coordinator_pb2.RendezvousResponse.LATER
            logger.info(
                "Reject participant",
                participant_id=participant_id,
                current_participants_count=self.participants.len(),
            )

        return coordinator_pb2.RendezvousReply(response=response)

    def _handle_heartbeat(
        self, _message: coordinator_pb2.HeartbeatRequest, participant_id: str
    ) -> coordinator_pb2.HeartbeatReply:
        """Handles a Heartbeat request.

        It checks if a participant has been selected, if it has,
        returns ROUND state to them, else STANDBY.

        Args:
            _message (:class:`~.coordinator_pb2.HeartbeatRequest`): The
                request to handle. Currently not used.
            participant_id (:obj:`str`): The id of the participant making the
                request.

        Returns:
            :class:`~.coordinator_pb2.HeartbeatReply`: The reply to the participant.
        """
        self.participants.update_expires(participant_id)

        if participant_id in self.round.participant_ids:
            state = coordinator_pb2.State.ROUND
        else:
            state = coordinator_pb2.State.STANDBY

        # send heartbeat reply advertising the current state
        return coordinator_pb2.HeartbeatReply(state=state, round=self.current_round)

    def _handle_start_training(
        self, _message: coordinator_pb2.StartTrainingRequest, participant_id: str
    ) -> coordinator_pb2.StartTrainingReply:
        """Handles a StartTraining request.

        Args:
            _message (:class:`~.coordinator_pb2.StartTrainingRequest`): The
                request to handle. Currently not used.
            participant_id (:obj:`str`): The id of the participant making the
                request.

        Returns:
            :class:`~.coordinator_pb2.StartTrainingReply`: The reply to the participant.
        """
        # The coordinator should only accept StartTraining requests if is
        # in the ROUND state and when the participant has been selected for the round.
        coordinator_not_in_a_round = self.state != coordinator_pb2.State.ROUND
        participant_not_selected = participant_id not in self.round.participant_ids
        if coordinator_not_in_a_round or participant_not_selected:
            raise InvalidRequestError(
                f"Participant {participant_id} sent a "
                "StartTrainingRequest outside of a round"
            )

        weights_proto = [ndarray_to_proto(nda) for nda in self.weights]

        return coordinator_pb2.StartTrainingReply(
            weights=weights_proto, epochs=self.epochs, epoch_base=self.epoch_base
        )

    def _handle_end_training(
        self, message: coordinator_pb2.EndTrainingRequest, participant_id: str
    ) -> coordinator_pb2.EndTrainingReply:
        """Handles a EndTraining request.

        Args:
            message (:class:`~.coordinator_pb2.EndTrainingRequest`): The request to handle.
            participant_id (:obj:`str`): The id of the participant making the request.

        Returns:
            :class:`~.coordinator_pb2.EndTrainingReply`: The reply to the participant.
        """

        # TODO: Ideally we want to know for which round the participant is
        # submitting the updates and raise an exception if it is the wrong
        # round.
        weights_proto, number_samples, metrics_proto = (
            message.weights,
            message.number_samples,
            message.metrics,
        )

        # record the request data
        weight_update: Tuple[List[ndarray], int] = (
            [proto_to_ndarray(pnda) for pnda in weights_proto],
            number_samples,
        )
        metrics: Dict[str, List[ndarray]] = {
            k: [proto_to_ndarray(v) for v in mv.metrics]
            for k, mv in metrics_proto.items()
        }
        self.round.add_updates(participant_id, weight_update, metrics)

        # The round is over. Run the aggregation
        if self.round.is_finished():
            logger.info(
                "Running aggregation for round", current_round=self.current_round
            )

            self.weights = self.aggregator.aggregate(self.round.get_weight_updates())

            # update the round or finish the training session
            if self.current_round == self.num_rounds:
                self.state = coordinator_pb2.State.FINISHED
            else:
                self.current_round += 1
                # reinitialize the round
                self.select_participant_ids_and_init_round()

        return coordinator_pb2.EndTrainingReply()


class CoordinatorGrpc(coordinator_pb2_grpc.CoordinatorServicer):
    """The Coordinator gRPC service.

    The main logic for the Coordinator is decoupled from gRPC and implemented in the
    :class:`~.Coordinator` class. The gRPC message only handles client requests
    and forwards the messages to :class:`~.Coordinator`.

    Args:
        coordinator (:class:`~.Coordinator`): The Coordinator state machine.
    """

    def __init__(self, coordinator: Coordinator):
        self.coordinator: Coordinator = coordinator

    def Rendezvous(
        self, request: coordinator_pb2.RendezvousRequest, context: grpc.ServicerContext
    ) -> coordinator_pb2.RendezvousReply:
        """The Rendezvous gRPC method.

        A participant contacts the coordinator and the coordinator adds the
        participant to its list of participants. If the coordinator already has
        all the participants it tells the participant to try again later.

        Args:
            request (:class:`~.coordinator_pb2.RendezvousRequest`): The participant's request.
            context (:class:`~grpc.ServicerContext`): The context associated with the gRPC request.

        Returns:
            :class:`~.coordinator_pb2.RendezvousReply`: The reply to the
            participant's request. The reply is an enum containing either:

                ACCEPT: If the :class:`~.Coordinator` does not have enough
                        participants.
                LATER: If the :class:`~.Coordinator` already has enough
                       participants.
        """
        return self.coordinator.on_message(request, context.peer())

    def Heartbeat(
        self, request: coordinator_pb2.HeartbeatRequest, context: grpc.ServicerContext
    ) -> coordinator_pb2.HeartbeatReply:
        """The Heartbeat gRPC method.

        Participants periodically send an heartbeat so that the
        :class:`Coordinator` can detect failures.

        Args:
            request (:class:`~.coordinator_pb2.HeartbeatRequest`): The
                participant's request. The participant's request contains the
                current :class:`~.coordinator_pb2.State` and round number the
                participant is on.
            context (:class:`~grpc.ServicerContext`): The context associated
                with the gRPC request.

        Returns:
            :class:`~.coordinator_pb2.HeartbeatReply`: The reply to the
            participant's request. The reply contains both the
            :class:`~.coordinator_pb2.State` and the current round the
            coordinator is on. If a training session has not started yet the
            round number defaults to 0.
        """
        try:
            return self.coordinator.on_message(request, context.peer())
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return coordinator_pb2.HeartbeatReply()

    def StartTraining(
        self,
        request: coordinator_pb2.StartTrainingRequest,
        context: grpc.ServicerContext,
    ) -> coordinator_pb2.StartTrainingReply:
        """The StartTraining gRPC method.

        Once a participant is notified that the :class:`~.Coordinator` is in a round
        (through the state advertised in the
        :class:`~.coordinator_pb2.HeartbeatReply`), the participant should call this
        method in order to get the global model weights in order to start the
        training for the round.

        Args:
            request (:class:`~.coordinator_pb2.StartTrainingRequest`): The participant's request.
            context (:class:`~grpc.ServicerContext`): The context associated with the gRPC request.

        Returns:
            :class:`~.coordinator_pb2.StartTrainingReply`: The reply to the
            participant's request. The reply contains the global model weights.
            """
        try:
            return self.coordinator.on_message(request, context.peer())
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return coordinator_pb2.StartTrainingReply()
        except InvalidRequestError as error:
            context.set_details(str(error))
            context.set_Code(grpc.StatusCode.FAILED_PRECONDITION)
            return coordinator_pb2.StartTrainingReply()

    def EndTraining(
        self, request: coordinator_pb2.EndTrainingRequest, context: grpc.ServicerContext
    ) -> coordinator_pb2.EndTrainingReply:
        """The EndTraining gRPC method.

        Once a participant has finished the training for the round it calls this
        method in order to submit to the :class:`~.Coordinator` the updated weights.

        Args:
            request (:class:`~.coordinator_pb2.EndTrainingRequest`): The
                participant's request. The request contains the updated weights as
                a result of the training as well as any metrics helpful for the
                :class:`~.Coordinator`.
            context (:class:`~grpc.ServicerContext`): The context associated with the gRPC request.

        Returns:
            :class:`~.coordinator_pb2.EndTrainingReply`: The reply to the
            participant's request. The reply is just an acknowledgment that
            the :class:`~.Coordinator` successfully received the updated
            weights.
        """
        try:
            return self.coordinator.on_message(request, context.peer())
        except DuplicatedUpdateError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.ALREADY_EXISTS)
            return coordinator_pb2.EndTrainingReply()
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return coordinator_pb2.EndTrainingReply()


def monitor_heartbeats(
    coordinator: Coordinator, terminate_event: threading.Event
) -> None:
    """Monitors the heartbeat of participants.

    If a heartbeat expires the participant is removed from the :class:`~.Participants`.

    Note:
        This is meant to be run inside a thread and expects an
        :class:`~threading.Event`, to know when it should terminate.

    Args:
        coordinator (:class:`~.Coordinator`): The coordinator to monitor for heartbeats.
        terminate_event (:class:`~threading.Event`): A threading event to signal
            that this method should terminate.
    """

    logger.info("Heartbeat monitor starting...")
    while not terminate_event.is_set():
        participants_to_remove: List[str] = []

        for participant in coordinator.participants.participants.values():
            if participant.heartbeat_expires < time.time():
                participants_to_remove.append(participant.participant_id)

        for participant_id in participants_to_remove:
            coordinator.remove_participant(participant_id)

        next_expiration: float = coordinator.participants.next_expiration() - time.time()

        logger.debug("Monitoring heartbeats", next_expiration=next_expiration)
        time.sleep(next_expiration)


def serve(coordinator: Coordinator, host: str = "[::]", port: int = 50051) -> None:
    """Main method to start the gRPC service.

    This methods just creates the :class:`~.Coordinator`, sets up all threading
    events and threads and configures and starts the gRPC service.
    """
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(coordinator, terminate_event)
    )

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator), server
    )
    server.add_insecure_port(f"{host}:{port}")
    server.start()
    monitor_thread.start()

    logger.info("Coordinator waiting for connections...")

    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        terminate_event.set()
        server.stop(0)
