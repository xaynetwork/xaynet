from concurrent import futures

import grpc
import numpy as np
import pytest
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc
from xain.grpc.numproto_server import NumProtoServer


@pytest.fixture
def greeter_server():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    hellonumproto_pb2_grpc.add_NumProtoServerServicer_to_server(
        NumProtoServer(), server
    )
    server.add_insecure_port("localhost:50051")
    server.start()
    yield
    server.stop(0)


# pylint: disable=W0613,W0621
@pytest.mark.integration
def test_greeter_server(greeter_server):
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = hellonumproto_pb2_grpc.NumProtoServerStub(channel)

        nda = np.arange(10)
        response = stub.SayHelloNumProto(
            hellonumproto_pb2.NumProtoRequest(arr=ndarray_to_proto(nda))
        )

        response_nda = proto_to_ndarray(response.arr)

        assert np.array_equal(nda * 2, response_nda)
