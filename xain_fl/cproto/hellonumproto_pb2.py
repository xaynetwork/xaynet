# -*- coding: utf-8 -*-
# Generated by the protocol buffer compiler.  DO NOT EDIT!
# source: xain_fl/grpc/hellonumproto.proto

import sys

_b = sys.version_info[0] < 3 and (lambda x: x) or (lambda x: x.encode("latin1"))
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf import reflection as _reflection
from google.protobuf import symbol_database as _symbol_database

# @@protoc_insertion_point(imports)

_sym_db = _symbol_database.Default()


from numproto.protobuf import ndarray_pb2 as numproto_dot_protobuf_dot_ndarray__pb2


DESCRIPTOR = _descriptor.FileDescriptor(
    name="xain_fl/grpc/hellonumproto.proto",
    package="hellonumproto",
    syntax="proto3",
    serialized_options=None,
    serialized_pb=_b(
        '\n xain_fl/grpc/hellonumproto.proto\x12\rhellonumproto\x1a\x1fnumproto/protobuf/ndarray.proto":\n\x0fNumProtoRequest\x12\'\n\x03\x61rr\x18\x01 \x01(\x0b\x32\x1a.numproto.protobuf.NDArray"8\n\rNumProtoReply\x12\'\n\x03\x61rr\x18\x01 \x01(\x0b\x32\x1a.numproto.protobuf.NDArray2d\n\x0eNumProtoServer\x12R\n\x10SayHelloNumProto\x12\x1e.hellonumproto.NumProtoRequest\x1a\x1c.hellonumproto.NumProtoReply"\x00\x62\x06proto3'
    ),
    dependencies=[numproto_dot_protobuf_dot_ndarray__pb2.DESCRIPTOR],
)


_NUMPROTOREQUEST = _descriptor.Descriptor(
    name="NumProtoRequest",
    full_name="hellonumproto.NumProtoRequest",
    filename=None,
    file=DESCRIPTOR,
    containing_type=None,
    fields=[
        _descriptor.FieldDescriptor(
            name="arr",
            full_name="hellonumproto.NumProtoRequest.arr",
            index=0,
            number=1,
            type=11,
            cpp_type=10,
            label=1,
            has_default_value=False,
            default_value=None,
            message_type=None,
            enum_type=None,
            containing_type=None,
            is_extension=False,
            extension_scope=None,
            serialized_options=None,
            file=DESCRIPTOR,
        )
    ],
    extensions=[],
    nested_types=[],
    enum_types=[],
    serialized_options=None,
    is_extendable=False,
    syntax="proto3",
    extension_ranges=[],
    oneofs=[],
    serialized_start=84,
    serialized_end=142,
)


_NUMPROTOREPLY = _descriptor.Descriptor(
    name="NumProtoReply",
    full_name="hellonumproto.NumProtoReply",
    filename=None,
    file=DESCRIPTOR,
    containing_type=None,
    fields=[
        _descriptor.FieldDescriptor(
            name="arr",
            full_name="hellonumproto.NumProtoReply.arr",
            index=0,
            number=1,
            type=11,
            cpp_type=10,
            label=1,
            has_default_value=False,
            default_value=None,
            message_type=None,
            enum_type=None,
            containing_type=None,
            is_extension=False,
            extension_scope=None,
            serialized_options=None,
            file=DESCRIPTOR,
        )
    ],
    extensions=[],
    nested_types=[],
    enum_types=[],
    serialized_options=None,
    is_extendable=False,
    syntax="proto3",
    extension_ranges=[],
    oneofs=[],
    serialized_start=144,
    serialized_end=200,
)

_NUMPROTOREQUEST.fields_by_name[
    "arr"
].message_type = numproto_dot_protobuf_dot_ndarray__pb2._NDARRAY
_NUMPROTOREPLY.fields_by_name[
    "arr"
].message_type = numproto_dot_protobuf_dot_ndarray__pb2._NDARRAY
DESCRIPTOR.message_types_by_name["NumProtoRequest"] = _NUMPROTOREQUEST
DESCRIPTOR.message_types_by_name["NumProtoReply"] = _NUMPROTOREPLY
_sym_db.RegisterFileDescriptor(DESCRIPTOR)

NumProtoRequest = _reflection.GeneratedProtocolMessageType(
    "NumProtoRequest",
    (_message.Message,),
    {
        "DESCRIPTOR": _NUMPROTOREQUEST,
        "__module__": "xain_fl.grpc.hellonumproto_pb2"
        # @@protoc_insertion_point(class_scope:hellonumproto.NumProtoRequest)
    },
)
_sym_db.RegisterMessage(NumProtoRequest)

NumProtoReply = _reflection.GeneratedProtocolMessageType(
    "NumProtoReply",
    (_message.Message,),
    {
        "DESCRIPTOR": _NUMPROTOREPLY,
        "__module__": "xain_fl.grpc.hellonumproto_pb2"
        # @@protoc_insertion_point(class_scope:hellonumproto.NumProtoReply)
    },
)
_sym_db.RegisterMessage(NumProtoReply)


_NUMPROTOSERVER = _descriptor.ServiceDescriptor(
    name="NumProtoServer",
    full_name="hellonumproto.NumProtoServer",
    file=DESCRIPTOR,
    index=0,
    serialized_options=None,
    serialized_start=202,
    serialized_end=302,
    methods=[
        _descriptor.MethodDescriptor(
            name="SayHelloNumProto",
            full_name="hellonumproto.NumProtoServer.SayHelloNumProto",
            index=0,
            containing_service=None,
            input_type=_NUMPROTOREQUEST,
            output_type=_NUMPROTOREPLY,
            serialized_options=None,
        )
    ],
)
_sym_db.RegisterServiceDescriptor(_NUMPROTOSERVER)

DESCRIPTOR.services_by_name["NumProtoServer"] = _NUMPROTOSERVER

# @@protoc_insertion_point(module_scope)
