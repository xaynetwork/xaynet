from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_600"
DATASET_SPLIT_HASHES = {
    "00": [
        "f6fe9532817309bf0843425fae64b8ba33bd2dcd",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "01": [
        "bd00e2d7d34d86f0f05bbd582c4f1334a0a48493",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "02": [
        "3ec316a095cfac4579b4ceb3798c338c3386802b",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "03": [
        "edef23ed8e9abe00ed9b54b019bab6dcaf045b0b",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "04": [
        "3aa473eedc1ef8fd6b2bc40bf63589de48fb9911",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "05": [
        "30d9bdec7d218d344f9fda60b6a474fa95af3fa7",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "06": [
        "cbed6164493e757a969fe9cbe8ebc714dd4c2566",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "07": [
        "cca25656c330b80731b88a403e325ca7e9fadcbf",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "08": [
        "e940b30caa7e952de93e1028f88b2a3f51f1d953",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "09": [
        "c3dd430e013049779732d651e8b05a7c027ae63c",
        "7c6f0d23624ac424b61c7757d8f6143cb418b7a6",
    ],
    "test": [
        "79e6584f3574e22e97dfe17ddf1b9856b3f2284f",
        "b056ffe622e9a6cfb76862c5ecb120c73d4fd99e",
    ],
    "val": [
        "c008c5dcaf03d23962756f92e6cd09a902f20f8b",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ],
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
