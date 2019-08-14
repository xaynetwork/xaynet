from typing import Tuple

import tensorflow as tf
from absl import flags

from autofl.datasets import storage, testing
from autofl.types import FederatedDataset

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_single_class"
DATASET_SPLIT_HASHES = {
    "00": [
        "f07bc648de06aa53bcb2237a9f5031398e42b5f7",
        "27fc2b251292942f840c30eeba766ffd6bd2d4f2",
    ],
    "01": [
        "a9e24938cb531fb5ec06b18ab3d1dbe22926350f",
        "bf675921ede2125de2d4c783a303511bbd1c2ab9",
    ],
    "02": [
        "e04d0f3ef0849d1565318c16abaa1f47bca9b7ef",
        "6ee9b9b5cf4c070bf81a2a1673eb45d7d66c77b8",
    ],
    "03": [
        "59cc38f595e284e7d579c59a538f50696a8ee809",
        "75aed8ca0d0070c2bb0cf208e603ba38c65d7e13",
    ],
    "04": [
        "e615f3a774192ecdb64aaa12b5b3b70444ea9c3d",
        "bda49e8cc8b463903b2c255ac643466c32609bf0",
    ],
    "05": [
        "20126348ad84389458fbd4d2944dbbb9e8337217",
        "a5819bce1f53108141782dd4550bbf9defa13b3c",
    ],
    "06": [
        "8cb60a2a330b12e333bff525c404720b746868ff",
        "98ba8b763570597da9de82fe32ae2425d9ba1296",
    ],
    "07": [
        "f39c4aa9750516b2cc17f894d9a28c1ba16b742d",
        "6425250faa8285d9f500913f449b60725cb5ab26",
    ],
    "08": [
        "d1cfe83e69a2db7e36d72e9e643fe066c87c766d",
        "93d6e6aca1eb539f2f6f8f8c2b1e1c120c53633b",
    ],
    "09": [
        "a5b5fa21b761e66b0eae54208c9b5b91c85f1054",
        "434c100e36be3e7d126e57fa1e7e76c814463a44",
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
    ds = storage.load_splits(
        dataset_name=DATASET_NAME,
        dataset_split_hashes=DATASET_SPLIT_HASHES,
        get_local_datasets_dir=get_local_datasets_dir,
    )

    testing.assert_dataset_origin(
        keras_dataset=testing.load(tf.keras.datasets.fashion_mnist),
        federated_dataset=ds,
    )

    return ds


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
