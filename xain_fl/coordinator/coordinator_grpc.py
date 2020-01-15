"""XAIN FL gRPC Coordinator"""

import grpc
from xain_proto.fl import coordinator_pb2, coordinator_pb2_grpc

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.store import Store
from xain_fl.tools.exceptions import (
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)


class CoordinatorGrpc(coordinator_pb2_grpc.CoordinatorServicer):
    """The Coordinator gRPC service.

    The main logic for the Coordinator is decoupled from gRPC and implemented in the
    :class:`xain_fl.coordinator.coordinator.Coordinator` class. The gRPC message only handles
    client requests and forwards the messages to
    :class:`xain_fl.coordinator.coordinator.Coordinator`.

    Args:
        coordinator (:class:`xain_fl.coordinator.coordinator.Coordinator`): The Coordinator
         state machine.

        store (:class:`xain_fl.coordinator.store.Store`): The Store in
            which the coordinator fetches trained models from the
            participants and to which it saves aggregated models.
    """

    def __init__(self, coordinator: Coordinator, store: Store):
        self.coordinator: Coordinator = coordinator
        self.store: Store = store

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

                ACCEPT: If the :class:`xain_fl.coordinator.coordinator.Coordinator`
                    does not have enough participants.
                LATER: If the :class:`xain_fl.coordinator.coordinator.Coordinator`
                    already has enough participants.
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
        self, request: coordinator_pb2.StartTrainingRequest, context: grpc.ServicerContext,
    ) -> coordinator_pb2.StartTrainingReply:
        """The StartTraining gRPC method.

        Once a participant is notified that the :class:`xain_fl.coordinator.coordinator.Coordinator`
        is in a round (through the state advertised in the
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
        method in order to submit to the :class:`xain_fl.coordinator.coordinator.Coordinator`
        the updated weights.

        Args:
            request (:class:`~.coordinator_pb2.EndTrainingRequest`): The
                participant's request. The request contains the updated weights as
                a result of the training as well as any metrics helpful for the
                :class:`xain_fl.coordinator.coordinator.Coordinator`.
            context (:class:`~grpc.ServicerContext`): The context associated with the gRPC request.

        Returns:
            :class:`~.coordinator_pb2.EndTrainingReply`: The reply to the
            participant's request. The reply is just an acknowledgment that
            the :class:`xain_fl.coordinator.coordinator.Coordinator` successfully received
            the updated weights.
        """
        try:
            response = self.coordinator.on_message(request, context.peer())
        except DuplicatedUpdateError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.ALREADY_EXISTS)
            return coordinator_pb2.EndTrainingReply()
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return coordinator_pb2.EndTrainingReply()

        if self.coordinator.state == coordinator_pb2.State.FINISHED:
            self.store.write_weights(self.coordinator.current_round, self.coordinator.weights)
        return response
