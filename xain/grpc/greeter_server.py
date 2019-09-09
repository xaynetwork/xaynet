# Copyright 2015 gRPC authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""The Python implementation of the GRPC helloworld.Greeter server."""

import logging
import time
from concurrent import futures

import grpc
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.grpc import hellonumproto_pb2, hellonumproto_pb2_grpc

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
