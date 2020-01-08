"""XAIN FL numproto server"""

from concurrent import futures
import time

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray
from xain_proto.fl import hellonumproto_pb2, hellonumproto_pb2_grpc

from xain_fl.coordinator import _ONE_DAY_IN_SECONDS
from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


class NumProtoServer(  # pylint: disable=too-few-public-methods
    hellonumproto_pb2_grpc.NumProtoServerServicer
):
    """[summary]

    [extended_summary]
    """

    def SayHelloNumProto(self, request, context):
        """[summary]

        [extended_summary]

        Args:
            request ([type]): [description]
            context ([type]): [description]

        Returns:
            [type]: [description]
        """

        nda = proto_to_ndarray(request.arr)
        logger.info("NumProto server received", nda=nda)

        nda *= 2
        logger.info("NumProto server sent", nda=nda)
        return hellonumproto_pb2.NumProtoReply(arr=ndarray_to_proto(nda))


def serve():
    """[summary]

    [extended_summary]
    """

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    hellonumproto_pb2_grpc.add_NumProtoServerServicer_to_server(NumProtoServer(), server)
    server.add_insecure_port("[::]:50051")
    server.start()
    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        server.stop(0)


if __name__ == "__main__":
    serve()
