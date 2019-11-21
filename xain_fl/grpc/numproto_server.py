import logging
import time
from concurrent import futures

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain_fl.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc

_ONE_DAY_IN_SECONDS = 60 * 60 * 24


class NumProtoServer(hellonumproto_pb2_grpc.NumProtoServerServicer):
    def SayHelloNumProto(self, request, context):
        nda = proto_to_ndarray(request.arr)
        print("NumProto server received: {}".format(nda))

        nda *= 2
        print("NumProto server sent: {}".format(nda))
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
    logging.basicConfig()
    serve()
