from concurrent import futures

import grpc
import pytest

from xain.grpc import helloworld_pb2, helloworld_pb2_grpc
from xain.grpc.greeter_server import Greeter


@pytest.fixture
def greeter_server():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    helloworld_pb2_grpc.add_GreeterServicer_to_server(Greeter(), server)
    server.add_insecure_port("localhost:50051")
    server.start()
    yield
    server.stop(0)


# pylint: disable=W0613,W0621
def test_greeter_server(greeter_server):
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = helloworld_pb2_grpc.GreeterStub(channel)
        response = stub.SayHello(helloworld_pb2.HelloRequest(name="xain"))

        assert response.message == "Hello, xain!"
