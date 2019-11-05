"""Module implementing the networked coordinator using gRPC.

This module implements the Coordinator state machine, the Coordinator gRPC
service and helper class to keep state about the Participants.
"""
import threading
import time
from concurrent import futures
from typing import Dict, List, Optional, Tuple

import grpc
import numpy as np
from google.protobuf.internal.python_message import GeneratedProtocolMessageType
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.fl.coordinator.aggregate import Aggregator, FederatedAveragingAgg
from xain.grpc import coordinator_pb2, coordinator_pb2_grpc

_ONE_DAY_IN_SECONDS = 60 * 60 * 24
HEARTBEAT_TIME = 10
HEARTBEAT_TIMEOUT = 5


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
        self.participant_id = participant_id
        self.heartbeat_expires = time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT


class Participants:
    """This class provides some useful methods to handle all the participants
    connected to a coordinator in a thread safe manner by protecting access to
    the participants list with a lock.
    """

    def __init__(self) -> None:
        self.participants: Dict[str, ParticipantContext] = {}
        self._lock = threading.Lock()

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


class Coordinator:
    """Class implementing the main Coordinator logic. It is implemented as a
    state machine that reacts to received messages.

    The states of the Coordinator are:
        STANDBY: The coordinator is in standby mode, typically when waiting for
        participants to connect. In this mode the only messages that the
        coordinator can receive are :class:`~.coordinator_pb2.RendezvousRequest`
        and :class:`~.coordinator_pb2.HeartbeatRequest`.

        ROUND: A round is currently in progress. During a round the only
        messages the coordinator can receive are
        :class:`~.coordinator_pb2.StartTrainingRequest` and
        :class:`~.coordinator_pb2.EndTrainingRequest`.

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
        required_participants(:obj:`int`, optional): The minimum number of
            participants required to perform a round. Defaults to 10.
        aggregator: (:class:`~.Aggregator`, optional): The type of aggregation
            to perform at the end of each round. Defaults to
            :class:`~.FederatedAveragingAgg`.
        theta (:obj:`list` of :class:`~numpy.ndarray`, optional): The weights of
            the global model. Defaults to [].
        epochs (:obj:`int`, optional): TODO.  Defaults to 0.
        epochs_base (:obj:`int`, optional): TODO. Defautls to 0.
    """

    # pylint: disable-msg=too-many-instance-attributes
    # pylint: disable-msg=dangerous-default-value
    def __init__(
        self,
        num_rounds: int = 10,
        required_participants: int = 10,
        aggregator: Optional[Aggregator] = None,
        theta: List[np.ndarray] = [],
        epochs: int = 0,
        epoch_base: int = 0,
    ) -> None:
        self.required_participants = required_participants
        self.participants = Participants()
        self.num_rounds = num_rounds
        self.aggregator = aggregator if aggregator else FederatedAveragingAgg()

        # global model
        self.theta = theta
        self.epochs = epochs
        self.epoch_base = epoch_base

        # local model
        self.theta_updates: List[Tuple[List[np.ndarray], int]] = []
        self.histories: List[Dict[str, List[float]]] = []
        self.metricss: List[Tuple[int, List[int]]] = []

        # state variables
        self.state = coordinator_pb2.State.STANDBY
        self.round = 0

    def on_message(
        self, message: GeneratedProtocolMessageType, peer_id: str
    ) -> GeneratedProtocolMessageType:
        """Coordinator method that implements the state machine.

        Args:
            message (:class:`~.GeneratedProtocolMessageType`): A protobuf message.
            peer_id (:obj:`str`): The id of the peer making the request.

        Returns:
            :class:`~.GeneratedProtocolMessageType`: The reply to be sent back to the peer.
        """
        print(f"Received: {type(message)} from {peer_id}")

        # pylint: disable-msg=no-else-return
        if isinstance(message, coordinator_pb2.RendezvousRequest):
            # Handle rendezvous

            if self.participants.len() < self.required_participants:
                response = coordinator_pb2.RendezvousResponse.ACCEPT
                self.participants.add(peer_id)
                print(
                    f"Accepted participant {peer_id}"
                    f" # participants: {self.participants.len()}"
                )

                # Change the state to ROUND if we are in STANDBY and already
                # have enough participants
                if self.participants.len() == self.required_participants:
                    # TODO: We may need to make this update thread safe
                    self.state = coordinator_pb2.State.ROUND
                    self.round = 1 if self.round == 0 else self.round
            else:
                response = coordinator_pb2.RendezvousResponse.LATER
                print(
                    f"Rejected participant {peer_id}"
                    f" # participants: {self.participants.len()}"
                )

            return coordinator_pb2.RendezvousReply(response=response)

        elif isinstance(message, coordinator_pb2.HeartbeatRequest):
            # Handle heartbeat

            self.participants.update_expires(peer_id)

            # send heartbeat reply advertising the current state
            return coordinator_pb2.HeartbeatReply(state=self.state, round=self.round)

        elif isinstance(message, coordinator_pb2.StartTrainingRequest):
            # handle start training

            # TODO: Check that the state == ROUND else raise exception

            theta_proto = [ndarray_to_proto(nda) for nda in self.theta]

            return coordinator_pb2.StartTrainingReply(
                theta=theta_proto, epochs=self.epochs, epoch_base=self.epoch_base
            )

        elif isinstance(message, coordinator_pb2.EndTrainingRequest):
            # handle end training

            tu, his, met = message.theta_update, message.history, message.metrics
            tp, num = tu.theta_prime, tu.num_examples
            cid, vbc = met.cid, met.vol_by_class
            # record the req data
            theta_update = [proto_to_ndarray(pnda) for pnda in tp], num
            self.theta_updates.append(theta_update)
            self.histories.append({k: list(hv.values) for k, hv in his.items()})
            self.metricss.append((cid, list(vbc)))

            # The round is over. Run the aggregation
            if len(self.theta_updates) == self.required_participants:
                print(f"Running aggregation for round {self.round}")
                self.theta = self.aggregator.aggregate(self.theta_updates)

                # update the round or finish the training session
                if self.round == self.num_rounds:
                    self.state = coordinator_pb2.State.FINISHED
                else:
                    self.round += 1

                    # reinitialize local models
                    self.theta_updates = []
                    self.histories = []
                    self.metricss = []

            return coordinator_pb2.EndTrainingReply()

        else:
            raise NotImplementedError


class CoordinatorGrpc(coordinator_pb2_grpc.CoordinatorServicer):
    """The Coordinator gRPC service.

    The main logic for the Coordinator is decoupled from gRPC and implemented in the
    :class:`~.Coordinator` class. The gRPC message only handles client requests
    and forwards the messages to :class:`~.Coordinator`.

    Args:
        coordinator (:class:`~.Coordinator`): The Coordinator state machine.

    """

    def __init__(self, coordinator: Coordinator):
        self.coordinator = coordinator

    def Rendezvous(
        self, request: coordinator_pb2.RendezvousRequest, context: grpc.ServicerContext
    ) -> coordinator_pb2.RendezvousReply:
        """The Rendezvous gRPC method.

        A participant contacts the coordinator and the coordinator adds the participant to
        its list of participants. If the coordinator already has all the participants it
        needs it tells the participant to try again later.

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
        return self.coordinator.on_message(request, context.peer())

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
        return self.coordinator.on_message(request, context.peer())

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
            participant's request. The reply is just and acknowledgment that
            the :class:`~.Coordinator` successfully received the updated
            weights.
        """
        return self.coordinator.on_message(request, context.peer())


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
        terminate_event (:class:`~threading.Event`): A threading even to signal
            that this method should terminate.
    """

    while not terminate_event.is_set():
        print("Monitoring heartbeats")
        participants_to_remove = []

        for participant in coordinator.participants.participants.values():
            if participant.heartbeat_expires < time.time():
                participants_to_remove.append(participant.participant_id)

        for participant_id in participants_to_remove:
            coordinator.participants.remove(participant_id)
            print(f"Removing participant {participant_id}")

        next_expiration = coordinator.participants.next_expiration() - time.time()

        print(f"Monitoring heartbeats in {next_expiration:.2f}s")
        time.sleep(next_expiration)


def serve() -> None:
    """Main method to start the gRPC service.

    This methods just creates the :class:`~.Coordinator`, setups all threading
    events and threads and configures and starts the gRPC service.
    """
    coordinator = Coordinator()
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(coordinator, terminate_event)
    )

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator), server
    )
    server.add_insecure_port("[::]:50051")
    server.start()
    monitor_thread.start()

    print("Coordinator waiting for connections...")

    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        terminate_event.set()
        server.stop(0)


if __name__ == "__main__":
    serve()
