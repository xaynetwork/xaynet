import threading
import time
from concurrent import futures
from typing import List, Tuple

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.grpc import coordinator_pb2, coordinator_pb2_grpc
from xain.types import History, Metrics, Theta

_ONE_DAY_IN_SECONDS = 60 * 60 * 24
HEARTBEAT_TIME = 10
HEARTBEAT_TIMEOUT = 5


class ParticipantContext:
    """Class to store state about each participant. Currently it only stores the `participant_id`
    and the time when the next heartbeat_expires.

    In the future we may store more information like in what state a participant is in e.g.
    IDLE, RUNNING, ...
    """

    def __init__(self, participant_id):
        self.participant_id = participant_id
        self.heartbeat_expires = time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT


class Participants:
    """This class provides some useful methods to handle all the participants connected to
    a coordinator in a thread safe manner by protecting access to the participants list with a
    lock.
    """

    def __init__(self):
        self.participants = {}
        self._lock = threading.Lock()

    def add(self, participant_id):
        """Adds a new participant to the list of participants.

        Args:
            participant_id (str): The id of the participant to add.
        """

        with self._lock:
            self.participants[participant_id] = ParticipantContext(participant_id)

    def remove(self, participant_id):
        """Removes a participant from the list of participants.

        This will be typically used after a participant is disconnected from the coordinator.

        Args:
            participant_id (str): The id of the participant to remove.
        """

        with self._lock:
            if participant_id in self.participants:
                del self.participants[participant_id]

    def next_expiration(self):
        """Helper method to check what is the next heartbeat to expire.

        Currently being used by the `heartbeat_monitor` to check how long it should sleep until
        the next check.

        Returns:
            float: The next heartbeat to expire or HEARTBEAT + HEARTBEAT_TIMEOUT if there are no
                no participants.
        """

        with self._lock:
            if self.participants:
                return min([p.heartbeat_expires for p in self.participants.values()])

        return time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT

    def len(self):
        """Get the number of participants.

        Returns:
            int: The number of participants in the list.
        """

        with self._lock:
            return len(self.participants)

    def update_expires(self, participant_id):
        """Updates the heartbeat expiration time for a participant.

        This is currently called by the `Coordinator` every time a participant sends a
        heartbeat.

        Args:
            participant_id (str): The id of the participant.
        """

        with self._lock:
            self.participants[participant_id].heartbeat_expires = (
                time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT
            )


def monitor_heartbeats(participants, terminate_event):
    """Monitors the heartbeat of participants.

    If a heartbeat expires the participant is removed from the list of participants.

    Note:
        This is meant to be run inside a thread and expects a `threading.Event` to know when
        it should terminate.

    Args:
        participants (Participants): The participants to monitor.
    """

    while not terminate_event.is_set():
        print("Monitoring heartbeats")
        participants_to_remove = []

        for participant in participants.participants.values():
            if participant.heartbeat_expires < time.time():
                participants_to_remove.append(participant.participant_id)

        for participant_id in participants_to_remove:
            participants.remove(participant_id)
            print(f"Removing participant {participant_id}")

        next_expiration = participants.next_expiration() - time.time()

        print(f"Monitoring heartbeats in {next_expiration:.2f}s")
        time.sleep(next_expiration)


class Coordinator(coordinator_pb2_grpc.CoordinatorServicer):
    def __init__(
        self,
        participants,
        required_participants=10,
        theta: Theta = [],
        epochs=0,
        epoch_base=0,
    ):
        self.required_participants = required_participants
        self.participants = participants
        # start-training reply data
        self.theta = theta
        self.epochs = epochs
        self.epoch_base = epoch_base
        # end-training request data
        self.theta_updates: List[Tuple[Theta, int]] = []
        self.histories: List[History] = []
        self.metricss: List[Metrics] = []

    def Rendezvous(self, request, context):
        if self.participants.len() < self.required_participants:
            response = coordinator_pb2.RendezvousResponse.ACCEPT
            self.participants.add(context.peer())
            print(
                f"Accepted participant {context.peer()}"
                f" # participants: {self.participants.len()}"
            )
        else:
            response = coordinator_pb2.RendezvousResponse.LATER
            print(
                f"Rejected participant {context.peer()}"
                f" # participants: {self.participants.len()}"
            )

        return coordinator_pb2.RendezvousReply(response=response)

    def Heartbeat(self, request, context):
        print(f"Received: {type(request)} from {context.peer()}")
        self.participants.update_expires(context.peer())
        return coordinator_pb2.HeartbeatReply()

    def StartTraining(self, request, context):
        print(f"Received: {type(request)} from {context.peer()}")
        # prepare reply
        theta_proto = [ndarray_to_proto(nda) for nda in self.theta]
        # send reply
        return coordinator_pb2.StartTrainingReply(
            theta=theta_proto, epochs=self.epochs, epoch_base=self.epoch_base
        )

    def EndTraining(self, request, context):
        print(f"Received: {type(request)} from {context.peer()}")
        tu, his, met = request.theta_update, request.history, request.metrics
        tp, num = tu.theta_prime, tu.num_examples
        cid, vbc = met.cid, met.vol_by_class
        # record the req data
        theta_update = [proto_to_ndarray(pnda) for pnda in tp], num
        self.theta_updates.append(theta_update)
        self.histories.append({k: list(hv.values) for k, hv in his.items()})
        self.metricss.append((cid, list(vbc)))
        # reply
        return coordinator_pb2.EndTrainingReply()


def serve():
    participants = Participants()
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(participants, terminate_event)
    )

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        Coordinator(participants), server
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
