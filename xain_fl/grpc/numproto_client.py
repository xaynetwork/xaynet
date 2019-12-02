import logging

import grpc
import numpy as np
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc


def run():
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = hellonumproto_pb2_grpc.NumProtoServerStub(channel)

        nda = np.arange(10)
        print("NumProto client sent: {}".format(nda))

        response = stub.SayHelloNumProto(
            hellonumproto_pb2.NumProtoRequest(arr=ndarray_to_proto(nda))
        )
    print("NumProto client received: {}".format(proto_to_ndarray(response.arr)))


if __name__ == "__main__":
    logging.basicConfig()
    run()
