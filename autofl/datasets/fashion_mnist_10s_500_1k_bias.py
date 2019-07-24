from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_500_1k_bias"
DATASET_SPLIT_HASHES = {
    "00": [
        "e75b8d92e5cce4ee00f79a19d73875232ca7b80e",
        "bfefa513f259b22deb0174dd56c56e8091552d8a",
    ],
    "01": [
        "2a1b97bb795f08e443430a6e0358d373d9c2748e",
        "cef088c6d1a013e80a22e963476a842ad23be954",
    ],
    "02": [
        "b8c3daf8181dc23edea698ccc011ca5a1b0e3ca6",
        "fb8e5c1b14d6cd511e8cc9ffb386cf590413dd01",
    ],
    "03": [
        "3f5bd1f873adef031bef9484968adaf19df44e1c",
        "f28bbb98f37ad145fa2adc790cae4e865a31d952",
    ],
    "04": [
        "d4ac6d1030416740e08c3577634060baaf3d12fa",
        "a94c0c6fee66de9f5c62a086a8eadad6e80e6f4d",
    ],
    "05": [
        "97ea01ca138c6c290fd633d38e5f8b87a7d67c06",
        "4a66f591bc3108df63a919afc8fc593febbbe2a0",
    ],
    "06": [
        "0f11e7cad7a67071361556d4d0c4e3325e3adf6a",
        "b4072ac861085e5343d1410b501508bf7748120d",
    ],
    "07": [
        "69515ea67d1594554ceffe3483368979d466ecf3",
        "80a95aab7338dce1fbc608900fb80829c1bd8589",
    ],
    "08": [
        "802c50071833ef525c376dce29d048377c4cca47",
        "b6c4e9b5647e70071f1c720d01785a83469feb52",
    ],
    "09": [
        "9446227a3d621c92f7e80fdca876ec869c513e20",
        "5bc1aa23a06ef8280146bb8377c2c5f31245cf25",
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
