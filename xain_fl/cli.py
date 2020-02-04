"""Module implementing the networked coordinator using gRPC.

This module implements the Coordinator state machine, the Coordinator gRPC
service and helper class to keep state about the Participants.
"""
import argparse

import numpy as np

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.store import Store, StoreConfig
from xain_fl.serve import serve


def type_num_rounds(value):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        value ([type]): [description]

    Returns:
        [type]: [description]

    Raises:
        ~argparse.ArgumentTypeError: [description]
        ~argparse.ArgumentTypeError: [description]
    """

    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 1_000:
        raise argparse.ArgumentTypeError("%s More than 1_000 rounds is not supported" % value)

    return ivalue


def type_num_epochs(value):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        value ([type]): [description]

    Returns:
        [type]: [description]

    Raises:
        ~argparse.ArgumentTypeError: [description]
        ~argparse.ArgumentTypeError: [description]
    """

    ivalue = int(value)

    if ivalue < 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 10_000:
        raise argparse.ArgumentTypeError("%s More than 10_000 epochs is not supported" % value)

    return ivalue


def type_min_num_participants_in_round(value):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        value ([type]): [description]

    Returns:
        [type]: [description]

    Raises:
        ~argparse.ArgumentTypeError: [description]
        ~argparse.ArgumentTypeError: [description]
    """

    ivalue = int(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive int value" % value)

    if ivalue > 1_000_000:
        raise argparse.ArgumentTypeError(
            "%s More than 1_000_000 participants is currently not supported" % value
        )

    return ivalue


def type_fraction(value):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        value ([type]): [description]

    Returns:
        [type]: [description]

    Raises:
        ~argparse.ArgumentTypeError: [description]
        ~argparse.ArgumentTypeError: [description]
    """

    ivalue = float(value)

    if ivalue <= 0:
        raise argparse.ArgumentTypeError("%s is an invalid positive float value" % value)

    if ivalue > 1:
        raise argparse.ArgumentTypeError(
            "%s is not a valid fraction of the total participant count." % value
        )

    return ivalue


def get_cmd_parameters():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Returns:
        [type]: [description]
    """

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
        default=1,
        type=type_num_rounds,
        help="Number of global rounds the model is going to be trained for.",
    )

    parser.add_argument(
        "-e",
        dest="num_epochs",
        default=1,
        type=type_num_epochs,
        help="Number of local epochs per round.",
    )

    parser.add_argument(
        "-p",
        dest="min_num_participants_in_round",
        default=1,
        type=type_min_num_participants_in_round,
        help="Minimum number of participants to be selected for a round.",
    )

    parser.add_argument(
        "-c",
        dest="fraction",
        default=1.0,
        type=type_fraction,
        help="Fraction of total clients that participate in a training round. \
            A float between 0 and 1",
    )

    parser.add_argument(
        "--storage-endpoint", required=True, type=str, help="URL to the storage service to use",
    )

    parser.add_argument(
        "--storage-bucket",
        required=True,
        type=str,
        help="Name of the bucket for storing the aggregated models",
    )

    parser.add_argument(
        "--storage-key-id",
        required=True,
        type=str,
        help="AWS access key ID to use to authenticate to the storage service",
    )

    parser.add_argument(
        "--storage-secret-access-key",
        required=True,
        type=str,
        help="AWS secret access to use to authenticate to the storage service",
    )
    return parser.parse_args()


def main():
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    parameters = get_cmd_parameters()

    coordinator = Coordinator(
        weights=list(np.load(parameters.file, allow_pickle=True)),
        num_rounds=parameters.num_rounds,
        epochs=parameters.num_epochs,
        minimum_participants_in_round=parameters.min_num_participants_in_round,
        fraction_of_participants=parameters.fraction,
    )

    store_config = StoreConfig(
        parameters.storage_endpoint,
        parameters.storage_key_id,
        parameters.storage_secret_access_key,
        parameters.storage_bucket,
    )
    store = Store(store_config)

    serve(coordinator=coordinator, store=store, host=parameters.host, port=parameters.port)


if __name__ == "__main__":
    main()
