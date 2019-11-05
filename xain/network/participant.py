import random
import threading
import time
from threading import Event
from uuid import uuid4

import grpc
from numproto import proto_to_ndarray

from xain.network import PORT, SERVER_ADDRESS, stream_pb2, stream_pb2_grpc


class ParticipantRequestQueue(object):
    def __init__(self):
        self._lock = threading.Lock()
        self._responses = []
        self._has_response_event = Event()
        self._is_closed = False

    def __iter__(self):
        return self

    def __next__(self):
        if self.is_empty() and self._is_closed:
            raise StopIteration()

        self._has_response_event.wait()

        res = self._responses.pop(0)  # FIFO

        if self.is_empty():
            self._has_response_event.clear()

        return res

    def __len__(self):
        return len(self._responses)

    def add_response(self, response):
        with self._lock:
            self._responses.append(response)
            self._has_response_event.set()

    def close(self):
        self._is_closed = True

    def reset(self):
        self.__init__()

    def is_empty(self):
        return len(self._responses) == 0


# @contextmanager
# def participant_request_queue()


class ParticipantClient(object):
    def __init__(self):
        """Initializer. 
           Creates a gRPC channel for connecting to the server.
           Adds the channel to the generated client stub.
        Arguments:
            None.
        
        Returns:
            None.
        """
        self.uuid = uuid4().hex
        self.rqueue = ParticipantRequestQueue()
        self.reconnect_in = None

        print(f"Starting participant and connecting to {SERVER_ADDRESS}:{PORT}")

        # Will use roughtly 10 file descriptors so make sure to reuse as much as
        # possible and e.g. do not create to many participants
        self.channel = grpc.insecure_channel(
            f"{SERVER_ADDRESS}:{PORT}",
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

        def on_channel_state_change(*args, **kwargs):
            print(*args, **kwargs)

        self.channel.subscribe(on_channel_state_change)

        self.stub = stream_pb2_grpc.CoordinatorStub(self.channel)

    def reset(self):
        self.rqueue.reset()

    def train(self):
        coordinator_messages_iterator = self.stub.Train(self.rqueue)

        self.rqueue.add_response(self.create_init_message())

        training_instruction_message = next(coordinator_messages_iterator)

        if training_instruction_message.HasField("reconnect_in"):
            self.reconnect_in = training_instruction_message.reconnect_in
            return

        if not training_instruction_message.HasField("train_config"):
            raise Exception("training_instruction must have train_config field")

        theta, epochs, epoch_base = training_config_from_message(
            training_instruction_message
        )

        for progress in do_some_ml_training(theta, epochs, epoch_base):
            self.rqueue.add_response(self.create_progress_message(progress))

        self.rqueue.add_response(self.create_result_message())

        self.rqueue.close()

        final_message = next(coordinator_messages_iterator)

        if final_message.HasField("reconnect_in"):
            self.reconnect_in = final_message.reconnect_in
            return

    def create_init_message(self):
        return stream_pb2.ParticipantMessage(uuid=self.uuid)

    def create_progress_message(self, progress):
        return stream_pb2.ParticipantMessage(uuid=self.uuid, progress=progress)

    def create_result_message(self):
        return stream_pb2.ParticipantMessage(uuid=self.uuid, progress=1000)

    def report_progress(self, progress):
        print(progress)


def training_config_from_message(msg):
    theta = [proto_to_ndarray(nda) for nda in msg.train_config.theta]
    epochs = msg.train_config.epochs
    epoch_base = msg.train_config.epoch_base

    return theta, epochs, epoch_base


def do_some_ml_training(theta, epochs, epoch_base):
    print(f"Going to train with epoch_base: {epoch_base} for {epochs} epochs")

    for i in range(0, 100, 10):
        time.sleep(random.random() * 0.001)  # simulate something to do
        yield (i + 1) / 100


def start_participant():
    participant = ParticipantClient()

    while True:
        participant.reset()
        participant.train()

        if participant.reconnect_in is None:
            break

        print(f"Reconnecting in {participant.reconnect_in}")
        time.sleep(participant.reconnect_in)
