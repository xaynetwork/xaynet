from concurrent import futures
from unittest import mock

import grpc
import numpy as np
import pytest
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.grpc import (
    coordinator_pb2,
    coordinator_pb2_grpc,
    hellonumproto_pb2,
    hellonumproto_pb2_grpc,
)
from xain.grpc.coordinator import Coordinator


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


@pytest.mark.integration
def test_participant_rendezvous_accept(participant_stub, coordinator_service):
    reply = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())

    assert reply.response == coordinator_pb2.RendezvousResponse.ACCEPT


def mocked_init(self, required_participants=10):
    """Sets `num_accepted_participants` to be the same as `required_participants` so that
    the coordinator tells the client to try later.
    """
    self.required_participants = 10
    self.num_accepted_participants = 10


# TODO: Fix test so it also runs correctly on macos
@pytest.mark.xfail
@pytest.mark.integration
@mock.patch("xain.grpc.coordinator.Coordinator.__init__", new=mocked_init)
def test_participant_rendezvous_later(participant_stub):

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(Coordinator(), server)
    server.add_insecure_port("localhost:50051")

    server.start()

    reply = participant_stub.Rendezvous(coordinator_pb2.RendezvousRequest())

    server.stop(0)

    assert reply.response == coordinator_pb2.RendezvousResponse.LATER
