import os

import grpc
import numpy as np
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc
from xain_fl.logger import get_logger

logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


def run():
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = hellonumproto_pb2_grpc.NumProtoServerStub(channel)

        nda = np.arange(10)
        logger.info("NumProto client sent: %s", nda)

        response = stub.SayHelloNumProto(
            hellonumproto_pb2.NumProtoRequest(arr=ndarray_to_proto(nda))
        )

    logger.info("NumProto client received: %s", proto_to_ndarray(response.arr))


if __name__ == "__main__":
    run()
