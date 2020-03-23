"""XAIN FL gRPC Coordinator"""

import grpc
from xain_proto.fl.coordinator_pb2 import (
    EndTrainingRoundRequest,
    EndTrainingRoundResponse,
    HeartbeatRequest,
    HeartbeatResponse,
    RendezvousRequest,
    RendezvousResponse,
    StartTrainingRoundRequest,
    StartTrainingRoundResponse,
)
from xain_proto.fl.coordinator_pb2_grpc import CoordinatorServicer

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.tools.exceptions import (
    DuplicatedUpdateError,
    InvalidRequestError,
    UnknownParticipantError,
)


class CoordinatorGrpc(CoordinatorServicer):
    """The Coordinator gRPC service.

    The main logic for the Coordinator is decoupled from gRPC and implemented in the
    :class:`xain_fl.coordinator.coordinator.Coordinator` class. The gRPC message only handles
    client requests and forwards the messages to
    :class:`xain_fl.coordinator.coordinator.Coordinator`.

    Args:

        coordinator: The Coordinator state machine.
    """

    def __init__(self, coordinator: Coordinator):
        self.coordinator: Coordinator = coordinator

    def Rendezvous(
        self, request: RendezvousRequest, context: grpc.ServicerContext
    ) -> RendezvousResponse:
        """The Rendezvous gRPC method.

        A participant contacts the coordinator and the coordinator adds the
        participant to its list of participants. If the coordinator already has
        all the participants it tells the participant to try again later.

        Args:

            request: The participant's request.
            context: The context associated with the gRPC request.

        Returns:

            The response to the participant's request. The response is
            an enum containing either:

                - `ACCEPT`: If the
                  :class:`xain_fl.coordinator.coordinator.Coordinator`
                  does not have enough participants.

                - `LATER`: If the
                    :class:`xain_fl.coordinator.coordinator.Coordinator`
                    already has enough participants.
        """
        return self.coordinator.on_message(request, context.peer())

    def Heartbeat(
        self, request: HeartbeatRequest, context: grpc.ServicerContext
    ) -> HeartbeatResponse:
        """The Heartbeat gRPC method.

        Participants periodically send an heartbeat so that the
        :class:`Coordinator` can detect failures.

        Args:

            request: The participant's request. The participant's
                request contains the current
                :class:`~.coordinator_pb2.State` and round number the
                participant is on.

            context: The context associated with the gRPC request.

        Returns:

            The response to the participant's request. The response
            contains both the :class:`~.coordinator_pb2.State` and the
            current round the coordinator is on. If a training session
            has not started yet the round number defaults to 0.
        """
        try:
            return self.coordinator.on_message(request, context.peer())
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return HeartbeatResponse()

    def StartTrainingRound(
        self, request: StartTrainingRoundRequest, context: grpc.ServicerContext,
    ) -> StartTrainingRoundResponse:
        """The StartTrainingRound gRPC method.

        Once a participant is notified that the :class:`xain_fl.coordinator.coordinator.Coordinator`
        is in a round (through the state advertised in the
        :class:`~.coordinator_pb2.HeartbeatResponse`), the participant should call this
        method in order to get the global model weights in order to start the
        training for the round.

        Args:

            request: The participant's request.
            context: The context associated with the gRPC request.

        Returns:

            The response to the participant's request. The response
            contains the global model weights.
        """
        try:
            return self.coordinator.on_message(request, context.peer())
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return StartTrainingRoundResponse()
        except InvalidRequestError as error:
            context.set_details(str(error))
            context.set_Code(grpc.StatusCode.FAILED_PRECONDITION)
            return StartTrainingRoundResponse()

    def EndTrainingRound(
        self, request: EndTrainingRoundRequest, context: grpc.ServicerContext
    ) -> EndTrainingRoundResponse:
        """The EndTrainingRound gRPC method.

        Once a participant has finished the training for the round it calls this
        method in order to submit to the :class:`xain_fl.coordinator.coordinator.Coordinator`
        the updated weights.

        Args:

            request: The participant's request. The request contains
                the updated weights as a result of the training as
                well as any metrics helpful for the
                :class:`xain_fl.coordinator.coordinator.Coordinator`.

            context: The context associated with the gRPC request.

        Returns:

            The response to the participant's request. The response is
            just an acknowledgment that the
            :class:`xain_fl.coordinator.coordinator.Coordinator`
            successfully received the updated weights.
        """
        try:
            return self.coordinator.on_message(request, context.peer())
        except DuplicatedUpdateError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.ALREADY_EXISTS)
            return EndTrainingRoundResponse()
        except UnknownParticipantError as error:
            context.set_details(str(error))
            context.set_code(grpc.StatusCode.PERMISSION_DENIED)
            return EndTrainingRoundResponse()
