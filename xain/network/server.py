import os
import threading
import time
from abc import ABC
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


class ClientProxy(ABC):
    """Proxy class for a class holding requests and awaiting responses"""

    def __init__(self):
        self.closed = False

        self.client_message = None
        self.server_message = None

        self.client_message_event = threading.Event()
        self.server_message_event = threading.Event()

    def _set_client_message(self, client_message):
        # set message and unblock client_message_event.wait() calls
        self.client_message = client_message
        self.client_message_event.set()

        # Clear server message so new instruction can be stored
        self.server_message = None
        self.server_message_event.clear()

    def _set_server_message(self, server_message):
        # set message and unblock server_message_event.wait() calls
        self.server_message = server_message
        self.server_message_event.set()

        # Clear client message so new response can be stored
        self.client_message = None
        self.client_message_event.clear()

    def process(self, client_message):
        """Starts processing of a client_message"""
        # Set client request
        self._set_client_message(client_message)

        # Await server message and store it as a return value
        self.server_message_event.wait()

        return self.server_message

    def close(self):
        if self.closed:
            raise Exception("ClientProxy is already closed")

        self.closed = True

    def run(
        self, instruction: stream_pb2.ServerMessage, skip_response=False
    ) -> Optional[stream_pb2.ClientMessage]:
        if self.closed:
            raise Exception("ClientProxy is already closed")

        # Set instruction as server message
        # print("Sending instruction")
        self._set_server_message(instruction)

        # print("Waiting for client message")

        if skip_response:
            return None

        # Wait for response from client
        self.client_message_event.wait()

        return self.client_message


class ClientManagerServicer(stream_pb2_grpc.ClientManagerServicer):
    # pylint: disable=too-many-instance-attributes
    def __init__(self, client_proxy_factory):
        self.client_proxy_factory = client_proxy_factory
        self.client_proxies = []

    def Connect(self, request_iterator, context):
        peer_id = context.peer()
        client_proxy = self.client_proxy_factory()
        self.client_proxies.append(client_proxy)

        print(f"Client {peer_id} connected")

        def rpc_termination_callback():
            if client_proxy in self.client_proxies:
                print(f"Delete peer {peer_id}")
                self.client_proxies.remove(client_proxy)

            print(f"Client {peer_id} disconnected")

        context.add_callback(rpc_termination_callback)

        for request in request_iterator:
            # Yielded proto message is send to client
            yield client_proxy.process(request)

    def get_clients(self, min_num_clients, check_interval=1):
        """Returns num_clients"""
        while True:
            open_client_proxies = [cp for cp in self.client_proxies if not cp.closed]
            num_connected_clients = len(open_client_proxies)

            if num_connected_clients >= min_num_clients:
                break

            time.sleep(check_interval)

        print(f"num_connected_clients: {num_connected_clients}/{min_num_clients}")

        return open_client_proxies


def keep_alive(server):
    try:
        while True:
            time.sleep(86400)
    except KeyboardInterrupt:
        server.stop(0)


def create_client_manager(
    client_proxy_factory, server_address=DEFAULT_SERVER_ADDRESS, port=DEFAULT_PORT
):
    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=1), maximum_concurrent_rpcs=10000
    )

    servicer = ClientManagerServicer(client_proxy_factory=client_proxy_factory)
    stream_pb2_grpc.add_ClientManagerServicer_to_server(servicer, server)

    server.add_insecure_port(f"{server_address}:{port}")

    server.start()
    # Pass server reference into thread where it will be kept alive
    threading.Thread(
        name="keep_alive(server)", target=keep_alive, args=(server,)
    ).start()

    print(f"Coordinator started. Listening at {server_address}:{port}.")
    print("Connection is insecure. No authentication enabled.")

    return servicer
