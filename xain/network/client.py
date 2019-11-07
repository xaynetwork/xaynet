import os
import threading
from contextlib import contextmanager
from threading import Event

import grpc

from xain.network import DEFAULT_PORT, DEFAULT_SERVER_ADDRESS, stream_pb2_grpc

os.environ["GRPC_VERBOSITY"] = "debug"
# os.environ["GRPC_TRACE"] = "connectivity_state"


class RequestQueue(object):
    """Queue with capacity for a single request"""

    def __init__(self):
        self._lock = threading.Lock()
        self._request = None
        self._has_request_event = Event()
        self._is_closed = False

    def __iter__(self):
        return self

    def __next__(self):
        if self._request and self._is_closed:
            raise StopIteration()

        self._has_request_event.wait()

        res = self._request

        self._request = None
        self._has_request_event.clear()

        return res

    def __len__(self):
        return 1 if self._request else 0

    def set_request(self, request):
        if self._request is not None:
            raise Exception("Can't set request before previous one is processed")

        with self._lock:
            self._request = request
            self._has_request_event.set()

    def close(self):
        self._is_closed = True

    def reset(self):
        self.__init__()

    def is_empty(self):
        return self._request is None


@contextmanager
def connection(server_address=DEFAULT_SERVER_ADDRESS, port=DEFAULT_PORT):
    channel = grpc.insecure_channel(
        f"{server_address}:{port}",
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

    channel.subscribe(on_channel_state_change)
    stub = stream_pb2_grpc.ClientManagerStub(channel)

    rqueue = RequestQueue()
    response_iterator = stub.Connect(rqueue)

    def consume():
        print("consume")
        return next(response_iterator)

    def dispatch(request):
        print("dispatch")
        rqueue.set_request(request)

    try:
        yield (consume, dispatch)
    finally:
        # Make sure to have a final
        rqueue.close()
        channel.close()
