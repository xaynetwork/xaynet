import os
import time
from concurrent import futures

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc
from xain_fl.logger import get_logger

_ONE_DAY_IN_SECONDS = 60 * 60 * 24
logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


class NumProtoServer(hellonumproto_pb2_grpc.NumProtoServerServicer):
    def SayHelloNumProto(self, request, context):
        nda = proto_to_ndarray(request.arr)
        logger.info("NumProto server received: %s", nda)

        nda *= 2
        logger.info("NumProto server sent: %s", nda)
        return hellonumproto_pb2.NumProtoReply(arr=ndarray_to_proto(nda))


def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    hellonumproto_pb2_grpc.add_NumProtoServerServicer_to_server(
        NumProtoServer(), server
    )
    server.add_insecure_port("[::]:50051")
    server.start()
    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        server.stop(0)


if __name__ == "__main__":
    serve()
