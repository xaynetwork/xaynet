import os
import threading
from contextlib import contextmanager
from threading import Event

import grpc

from xain.network import DEFAULT_PORT, DEFAULT_SERVER_ADDRESS, stream_pb2_grpc

os.environ["GRPC_VERBOSITY"] = "debug"
# os.environ["GRPC_TRACE"] = "connectivity_state"


class MessageQueue:
    """Queue with capacity for a single request"""

    def __init__(self):
        self._lock = threading.Lock()
        self._message = None
        self._has_message_event = Event()
        self._is_closed = False

    def __iter__(self):
        return self

    def __next__(self):
        if self._message and self._is_closed:
            raise StopIteration()

        self._has_message_event.wait()

        res = self._message

        self._message = None
        self._has_message_event.clear()

        return res

    def __len__(self):
        return 1 if self._message else 0

    def set_message(self, request):
        if self._message is not None:
            raise Exception("Can't set request before previous one is processed")

        with self._lock:
            self._message = request
            self._has_message_event.set()

    def close(self):
        self._is_closed = True

    def reset(self):
        self.__init__()

    def is_empty(self):
        return self._message is None


@contextmanager
def connection(server_address=DEFAULT_SERVER_ADDRESS, port=DEFAULT_PORT):
    channel = grpc.insecure_channel(f"{server_address}:{port}")

    def on_channel_state_change(*args, **kwargs):
        print(*args, **kwargs)

    channel.subscribe(on_channel_state_change)

    stub = stream_pb2_grpc.ParticipantManagerStub(channel)

    mqueue = MessageQueue()
    response_iterator = stub.Connect(mqueue)

    def consume():
        print("consume")
        return next(response_iterator)

    def dispatch(request):
        print("dispatch")
        mqueue.set_message(request)

    try:
        yield (consume, dispatch)
    finally:
        # Make sure to have a final
        mqueue.close()
        channel.close()
