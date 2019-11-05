import logging
import os
import random
import threading
import time
from concurrent import futures
from uuid import uuid4

import grpc
import numpy as np
from numproto import ndarray_to_proto

from xain.network import PORT, SERVER_ADDRESS, stream_pb2, stream_pb2_grpc

# gRPC Debug Settings

# GRPC_VERBOSITY Default gRPC logging verbosity - one of:
#     DEBUG - log all gRPC messages
#     INFO - log INFO and ERROR message
#     ERROR - log only errors

# GRPC_TRACE A comma separated list of tracers that provide additional insight into how gRPC C core is processing requests via debug logs. Available tracers include:
#     api - traces api calls to the C core
#     bdp_estimator - traces behavior of bdp estimation logic
#     call_error - traces the possible errors contributing to final call status
#     cares_resolver - traces operations of the c-ares based DNS resolver
#     cares_address_sorting - traces operations of the c-ares based DNS resolver's resolved address sorter
#     channel - traces operations on the C core channel stack
#     client_channel_call - traces client channel call batch activity
#     client_channel_routing - traces client channel call routing, including resolver and load balancing policy interaction
#     compression - traces compression operations
#     connectivity_state - traces connectivity state changes to channels
#     cronet - traces state in the cronet transport engine
#     executor - traces grpc's internal thread pool ('the executor')
#     glb - traces the grpclb load balancer
#     handshaker - traces handshaking state
#     health_check_client - traces health checking client code
#     http - traces state in the http2 transport engine
#     http2_stream_state - traces all http2 stream state mutations.
#     http1 - traces HTTP/1.x operations performed by gRPC
#     inproc - traces the in-process transport
#     flowctl - traces http2 flow control
#     op_failure - traces error information when failure is pushed onto a completion queue
#     pick_first - traces the pick first load balancing policy
#     plugin_credentials - traces plugin credentials
#     pollable_refcount - traces reference counting of 'pollable' objects (only in DEBUG)
#     resource_quota - trace resource quota objects internals
#     round_robin - traces the round_robin load balancing policy
#     queue_pluck
#     server_channel - lightweight trace of significant server channel events
#     secure_endpoint - traces bytes flowing through encrypted channels
#     subchannel - traces the connectivity state of subchannel
#     timer - timers (alarms) in the grpc internals
#     timer_check - more detailed trace of timer logic in grpc internals
#     transport_security - traces metadata about secure channel establishment
#     tcp - traces bytes in and out of a channel
#     tsi - traces tsi transport security


os.environ["GRPC_VERBOSITY"] = "debug"
os.environ["GRPC_TRACE"] = "connectivity_state"


class CoordinatorServicer(stream_pb2_grpc.CoordinatorServicer):
    # pylint: disable=too-many-instance-attributes
    def __init__(self):
        self.participants = {}
        self.uuid = uuid4().hex
        self.num_rounds = 10
        self.num_participants = 10
        self.num_messages = 0

        self.E = 10
        self.C = 0.2  # => required participants per round = 2

        self.required_participants = int(self.num_participants * self.C)

        self.current_round = 0

        # Should always be self.num_participants * self.C
        self.num_connected_participants = 0

        self.theta = [np.ones((1, 10))]

    def Train(self, request_iterator, context):
        def rpc_termination_callback():
            # When connection is shut down reduce number of connected participants
            self.num_connected_participants -= 1
            print(f"Participants disconnected: {self.num_connected_participants}")

        context.add_callback(rpc_termination_callback)

        self.num_connected_participants += 1

        participant_init_message = next(request_iterator)
        self.set_participant_last_seen(participant_init_message)

        log_msg = f"Participants connected: {self.num_connected_participants}/{self.required_participants}"
        print(log_msg)

        if self.num_connected_participants > self.required_participants:
            # now we assume ideal time = 10
            yield self.create_reconnect_in_instruction()
            return

        # Wait for all participants to be connected
        self.hold_connection_until_all_ready()

        print(f"Starting training with {self.num_connected_participants} participants")

        # Start training as we have enough participants connected
        yield self.create_training_instruction()

        for msg in request_iterator:
            self.num_messages += 1
            print(self.num_messages, msg.uuid, msg.progress)
            # block_random_time(up_to_secs=0.1)

        # IMPORTANT:
        # A final message is needed as the client will only hold the connection
        # open as long as there are outstanding messages in the server to be send
        # After the final yield the client will immidiatly close its request_iterator
        # even if the server did not consume all requests yet
        yield self.create_reconnect_in_instruction()

    def set_participant_last_seen(self, participant):
        if participant.uuid not in self.participants:
            self.participants[participant.uuid] = {}

        self.participants[participant.uuid]["last_seen"] = time.time()

    def hold_connection_until_all_ready(self):
        while True:
            if self.num_connected_participants == self.required_participants:
                break
            time.sleep(0.1)

        # Needed so each loop can check counts
        time.sleep(0.1)

    def create_reconnect_in_instruction(self):
        secs = int(random.random() * 2)
        return stream_pb2.CoordinatorMessage(reconnect_in=secs)

    def create_training_instruction(self):
        train_config = stream_pb2.CoordinatorMessage.TrainConfig(
            theta=[ndarray_to_proto(nda) for nda in self.theta],
            epochs=self.E,
            epoch_base=self.current_round * self.E,
        )
        return stream_pb2.CoordinatorMessage(train_config=train_config)


def block_random_time(up_to_secs):
    time.sleep(random.random() * up_to_secs)


def start_coordinator():
    terminate_event = threading.Event()
    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=10),
        options=(
            # ("grpc.keepalive_time_ms", 1000 * 30),
            # send keepalive ping every 30 min, default is 2 hours
            # ("grpc.keepalive_timeout_ms", 5000),
            # keepalive ping time out after 5 seconds, default is 20 seoncds
            # ("grpc.keepalive_permit_without_calls", True),
            # allow keepalive pings when there's no gRPC calls
            # ("grpc.http2.max_pings_without_data", 0),
            # allow unlimited amount of keepalive pings without data
            # ("grpc.http2.min_time_between_pings_ms", 10000),
            # allow grpc pings from client every 10 seconds
            # ("grpc.http2.min_ping_interval_without_data_ms", 5000),
            # allow grpc pings from client without data every 5 seconds
        ),
    )

    # Add Servicers to server
    stream_pb2_grpc.add_CoordinatorServicer_to_server(CoordinatorServicer(), server)

    server.add_insecure_port(f"{SERVER_ADDRESS}:{PORT}")
    server.start()

    print(f"Coordinator started. Listening at {SERVER_ADDRESS}:{PORT}.")
    print("Connection is insecure. No authentication enabled.")

    try:
        while True:
            time.sleep(60)
    except KeyboardInterrupt:
        terminate_event.set()
        server.stop(0)


if __name__ == "__main__":
    logging.basicConfig()
    start_coordinator()
