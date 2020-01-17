"""XAIN FL numproto client"""

import grpc
import numpy as np
from xain_proto.fl import hellonumproto_pb2, hellonumproto_pb2_grpc
from xain_proto.numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


def run():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    with grpc.insecure_channel("localhost:50051") as channel:
        stub = hellonumproto_pb2_grpc.NumProtoServerStub(channel)

        nda = np.arange(10)
        logger.info("NumProto client sent", nda=nda)

        response = stub.SayHelloNumProto(
            hellonumproto_pb2.NumProtoRequest(arr=ndarray_to_proto(nda))
        )

    logger.info("NumProto client received", nda=proto_to_ndarray(response.arr))


if __name__ == "__main__":
    run()
