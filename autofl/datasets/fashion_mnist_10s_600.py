# pylint: disable=R0801
from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_600"
DATASET_SPLIT_HASHES = {
    "00": (
        "c008c5dcaf03d23962756f92e6cd09a902f20f8b",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "01": (
        "d403feaf91c7f4e896a20dc5408100fad5862842",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "02": (
        "9fa8e9995565c523943180dcc99eb0fca4d571e8",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "03": (
        "eb719b95a6f77c9426e1850756e32363babe968f",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "04": (
        "bb2656915fe174924fc91f0b84e6496d169a55a8",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "05": (
        "6c01cd10d0b6285792826cac09b2e217a0810e40",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "06": (
        "a850b38f94196ee00633db97b97476f0ab91ea2c",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "07": (
        "f4361082c4953046dedf669275da75819b9312f5",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "08": (
        "597154fb56f18a585dca5372e53fd403465cac54",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "09": (
        "536d42f71f875f34003bd2a89ea2a886dff1a813",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ),
    "test": (
        "79e6584f3574e22e97dfe17ddf1b9856b3f2284f",
        "b056ffe622e9a6cfb76862c5ecb120c73d4fd99e",
    ),
}


def load_splits(
    get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    return storage.load_splits(
        dataset_name=DATASET_NAME,
        dataset_split_hashes=DATASET_SPLIT_HASHES,
        get_local_datasets_dir=get_local_datasets_dir,
    )


def load_split(
    split_id: str,
    split_hashes: Tuple[str, str],
    get_local_datasets_dir=storage.default_get_local_datasets_dir,
):
    assert split_id in set(DATASET_SPLIT_HASHES.keys())

    x_i, y_i = storage.load_split(
        dataset_name=DATASET_NAME,
        split_id=split_id,
        split_hashes=split_hashes,
        local_datasets_dir=get_local_datasets_dir(),
    )

    return x_i, y_i
