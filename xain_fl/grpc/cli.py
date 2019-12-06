"""Module implementing the networked coordinator using gRPC.

This module implements the Coordinator state machine, the Coordinator gRPC
service and helper class to keep state about the Participants.
"""
import argparse

import numpy as np

from xain_fl.grpc.coordinator import Coordinator, serve


def type_num_rounds(value):
    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 1_000:
        raise argparse.ArgumentTypeError(
            "%s More than 1_000 rounds is not supported" % value
        )

    return ivalue


def type_num_epochs(value):
    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 10_000:
        raise argparse.ArgumentTypeError(
            "%s More than 10_000 epochs is not supported" % value
        )

    return ivalue


def type_num_participants(value):
    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 1_000_000:
        raise argparse.ArgumentTypeError(
            "%s More than 1_000_000 participants is currently not supported" % value
        )

    return ivalue


def type_fraction(value):
    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError(
            "%s is an invalid positive float value" % value
        )

    if ivalue > 1:
        raise argparse.ArgumentTypeError(
            "%s is not a valid fraction of the total participant count." % value
        )

    return ivalue


def get_cmd_parameters():
    # Allow various parameters to be passed via the commandline
    parser = argparse.ArgumentParser(description="Coordinator CLI")

    parser.add_argument("--host", dest="host", default="[::]", type=str, help="Host")
    parser.add_argument("--port", dest="port", default=50051, type=int, help="Port")

    parser.add_argument(
        "-f",
        dest="file",
        required=True,
        help="Path to numpy ndarray file containing model weights",
    )

    parser.add_argument(
        "-r",
        dest="num_rounds",
        default=10,
        type=type_num_rounds,
        help="Number of global rounds the model is going to be trained for.",
    )

    parser.add_argument(
        "-e",
        dest="num_epochs",
        default=2,
        type=type_num_epochs,
        help="Number of local epochs per round.",
    )

    parser.add_argument(
        "-p",
        dest="num_participants",
        default=100,
        type=type_num_participants,
        help="Number of participants.",
    )

    parser.add_argument(
        "-c",
        dest="fraction",
        default=0.1,
        type=type_fraction,
        help="Fraction of total clients that participate in a training round. \
            A float between 0 and 1",
    )

    return parser.parse_args()


def main():
    parameters = get_cmd_parameters()

    coordinator = Coordinator(
        theta=list(np.load(parameters.file, allow_pickle=True)),
        num_rounds=parameters.num_rounds,
        epochs=parameters.num_epochs,
        required_participants=int(parameters.num_participants * parameters.fraction),
    )

    serve(coordinator=coordinator, host=parameters.host, port=parameters.port)


if __name__ == "__main__":
    main()
