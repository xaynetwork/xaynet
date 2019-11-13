import os
import threading
import time
from concurrent import futures
from typing import Optional

import grpc

from xain.network import (
    DEFAULT_PORT,
    DEFAULT_SERVER_ADDRESS,
    stream_pb2,
    stream_pb2_grpc,
)

os.environ["GRPC_VERBOSITY"] = "debug"
# os.environ["GRPC_TRACE"] = "connectivity_state"


class ParticipantProxy:
    """Proxy class for a class holding requests and awaiting responses"""

    def __init__(self):
        self.closed = False

        self.participant_message = None
        self.server_message = None

        self.participant_message_event = threading.Event()
        self.coordinator_message_event = threading.Event()

    def _set_participant_message(self, participant_message):
        # set message and unblock participant_message_event.wait() calls
        self.participant_message = participant_message
        self.participant_message_event.set()

        # Clear server message so new instruction can be stored
        self.server_message = None
        self.coordinator_message_event.clear()

    def _set_coordinator_message(self, server_message):
        """Sets coordinator message and unblocks all wait() calls on
        coordinator_message_event. Also clears participant message and
        blocks all wait calls for participant_message_event by clearing it
        """
        # Set message and unblock coordinator_message_event.wait() calls
        self.server_message = server_message
        self.coordinator_message_event.set()

        # Clear participant message so new response can be stored
        self.participant_message = None
        self.participant_message_event.clear()

    def process(self, participant_message):
        """Sets participant message for processing and awaits
        coordinator message to be returned"""
        # Set participant request
        self._set_participant_message(participant_message)

        # Await server message and store it as a return value
        self.coordinator_message_event.wait()

        return self.server_message

    def close(self):
        """Closes the proxy and unblocks it. This method is idempotent. If the
        proxy is alredy closed nothing further will happen"""
        # Close and
        self.closed = True

        # Unblock threads
        self.participant_message_event.set()
        self.coordinator_message_event.set()

    def run(
        self, instruction: stream_pb2.CoordinatorMessage, skip_response=False
    ) -> Optional[stream_pb2.ParticipantMessage]:
        if self.closed:
            raise Exception("ParticipantProxy is already closed")

        # Set instruction as coordinator message
        # print("Sending instruction")
        self._set_coordinator_message(instruction)

        # print("Waiting for participant message")

        if skip_response:
            return None

        # Wait for response from participant
        self.participant_message_event.wait()

        return self.participant_message


class ParticipantManager(stream_pb2_grpc.ParticipantManagerServicer):
    # pylint: disable=too-many-instance-attributes
    def __init__(self, participant_factory):
        self.participant_factory = participant_factory
        self.participants = []

    def Connect(self, request_iterator, context):
        peer_id = context.peer()
        participant = self.participant_factory()
        self.participants.append(participant)

        print(f"Participant {peer_id} connected ({len(self.participants)})")

        def rpc_termination_callback():
            print(f"Participant {peer_id} disconnected")

            participant.proxy.close()
            self.participants.remove(participant)

        context.add_callback(rpc_termination_callback)

        for request in request_iterator:
            # Yielded proto message is send to client
            yield participant.proxy.process(request)

    def has_enough_participants(self, min_num_participants):
        open_participant_proxies = [p for p in self.participants if not p.proxy.closed]
        num_connected_participants = len(open_participant_proxies)
        return num_connected_participants >= min_num_participants

    def get_participants(self, min_num_participants, check_interval=1):
        """Returns min_num_participants participants"""
        while not self.has_enough_participants(min_num_participants):
            time.sleep(1)

        open_participant_proxies = [p for p in self.participants if not p.proxy.closed]
        return open_participant_proxies


def keep_alive(server):
    try:
        while True:
            time.sleep(86400)
    except KeyboardInterrupt:
        server.stop(0)


def create_participant_manager(
    participant_factory, server_address=DEFAULT_SERVER_ADDRESS, port=DEFAULT_PORT
):
    """Creates a participant manager instance inside a gRPC server and returns it

    Args:
        participant_factory (Callable): Function which returns a participant instance
        server_address (string): host name of server as string e.g. "[::]"
        port (int): server port

    Returns:
        servicer (ParticipantManager): Instance of participant manager
    """

    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=100), maximum_concurrent_rpcs=200
    )
    """'max_workers' will set the number of parallel threads possible
    'maximum_concurrent_rpcs' will set the number of concurrent threads meaning
    if we are the 'max_workers' limit the connection of new streams will be blocked
    until threads become free. 'maximum_concurrent_rpcs - max_workers' will basically
    be the waiting line for incomming threads. Any incomming connection after the
    waiting line is full will be rejected
    """
    servicer = ParticipantManager(participant_factory=participant_factory)
    stream_pb2_grpc.add_ParticipantManagerServicer_to_server(servicer, server)

    server.add_insecure_port(f"{server_address}:{port}")

    server.start()
    # Pass server reference into thread where it will be kept alive
    threading.Thread(
        name="keep_alive(server)", target=keep_alive, args=(server,)
    ).start()

    print(f"Coordinator started. Listening at {server_address}:{port}.")
    print("Connection is insecure. No authentication enabled.")

    return servicer
