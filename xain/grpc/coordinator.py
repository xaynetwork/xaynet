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

# States
STANDBY = 0
ROUND = 1
FINISHED = 2
READY = 3
TRAINING = 4


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


def monitor_heartbeats(coordinator, terminate_event):
    """Monitors the heartbeat of participants.

    If a heartbeat expires the participant is removed from the list of participants.

    Note:
        This is meant to be run inside a thread and expects a `threading.Event` to know when
        it should terminate.

    Args:
        participants (Coordinator): The participants to monitor.
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


class CoordinatorGrpc(coordinator_pb2_grpc.CoordinatorServicer):
    def __init__(self, coordinator):
        self.coordinator = coordinator

    def Rendezvous(self, request, context):
        return self.coordinator.on_message(request, context.peer())

    def Heartbeat(self, request, context):
        return self.coordinator.on_message(request, context.peer())

    def StartTraining(self, request, context):
        return self.coordinator.on_message(request, context.peer())

    def EndTraining(self, request, context):
        return self.coordinator.on_message(request, context.peer())


class Coordinator:
    def __init__(self, required_participants=10, theta=None, epochs=0, epoch_base=0):
        self.required_participants = required_participants
        self.participants = Participants()

        # global model
        self.theta = [] if theta is None else theta
        self.epochs = epochs
        self.epoch_base = epoch_base

        # local model
        self.theta_updates = []
        self.histories = []
        self.metricss = []

        # state variables
        self.state = STANDBY
        self.round = 0

    def on_message(self, message, peer_id):
        print(f"Received: {type(message)} from {peer_id}")

        if type(message) == coordinator_pb2.RendezvousRequest:
            # Handle rendezvous

            if self.participants.len() < self.required_participants:
                response = coordinator_pb2.RendezvousResponse.ACCEPT
                self.participants.add(peer_id)
                print(
                    f"Accepted participant {peer_id}"
                    f" # participants: {self.participants.len()}"
                )
            else:
                # If state is STANDBY start a round

                response = coordinator_pb2.RendezvousResponse.LATER
                print(
                    f"Rejected participant {peer_id}"
                    f" # participants: {self.participants.len()}"
                )

            # Change the state to ROUND if we are in STANDBY and already have enough participants
            if (
                self.state == STANDBY
                and self.participants.len() == self.required_participants
            ):
                # TODO: We may need to make this update thread safe
                self.state = ROUND

            return coordinator_pb2.RendezvousReply(response=response)

        elif type(message) == coordinator_pb2.HeartbeatRequest:
            # Handle heartbeat

            self.participants.update_expires(peer_id)

            # send heartbeat reply advertising the current state
            return coordinator_pb2.HeartbeatReply(state=self.state, round=self.round)

        elif type(message) == coordinator_pb2.StartTrainingRequest:
            # handle start training

            # TODO: Check that the state == ROUND else raise exception
            # TODO: Update the round number

            theta_proto = [ndarray_to_proto(nda) for nda in self.theta]

            return coordinator_pb2.StartTrainingReply(
                theta=theta_proto, epochs=self.epochs, epoch_base=self.epoch_base
            )

        elif type(message) == coordinator_pb2.EndTrainingRequest:
            # handle end training

            tu, his, met = message.theta_update, message.history, message.metrics
            tp, num = tu.theta_prime, tu.num_examples
            cid, vbc = met.cid, met.vol_by_class
            # record the req data
            theta_update = [proto_to_ndarray(pnda) for pnda in tp], num
            self.theta_updates.append(theta_update)
            self.histories.append({k: list(hv.values) for k, hv in his.items()})
            self.metricss.append((cid, list(vbc)))

            # TODO: We need to check if all required participants called this method.
            #       If so we need to run the aggregation and start a new round or
            #       finish the training session

            return coordinator_pb2.EndTrainingReply()


def serve():
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
