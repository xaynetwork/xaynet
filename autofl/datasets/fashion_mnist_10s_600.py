# pylint: disable=R0801
from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_600_per_class_10.py"
# TODO: correct hashes
DATASET_SPLIT_HASHES = {
    "00": (
        "123d302de44a32a31a6c2930b053684ecd8b42cb681",
        "e3f701234ced3290d35cf4b2b5b59a04267ddb2c985",
    ),
    "01": (
        "123c15f47ee6952558bc679866a49dbf9671d1c3de0",
        "e5ad81232230e725e7c130ea30f968b5c5827069005",
    ),
    "02": (
        "1233d1043525d1661f9867fcdfd885d74877154d1c6",
        "17aec123fe65967a3af1e1e56850a46f30cf994c7a2",
    ),
    "03": (
        "12336cd5d81f399a94251da0230c7dce6dfbcde08ca",
        "3ff2e123aaa878fdabdec881a8c56ee359b41a5058d",
    ),
    "04": (
        "123bbdf9c8e8fb3caff23016a22b9b7fd823a79b96e",
        "e3217123dbbacb655b008d82f21dfe7ac0fd77a5e42",
    ),
    "05": (
        "123ab028c7e44c8e1786aa9ca13a463bbd2219ddb7d",
        "8e85b12362f14cc3ea8562d6de225caf2e5621e6871",
    ),
    "06": (
        "123b4e5e7360682f3708cffba852ba8fb819cfa919c",
        "a5c7412333c5ec57f8a79704c6cc5bcb63dfd733b5c",
    ),
    "07": (
        "123ab24961a27c02e509d3589737b42f44be85b4b59",
        "7e2e41231829d7fd0e72ba9824ded5fcc0fa51ae671",
    ),
    "08": (
        "123277c169c7a71e78a2c726e6d7558b614778f2036",
        "2fa72123d6d81bb2713bd2c50430b063b38c8b0bf07",
    ),
    "09": (
        "123e0067be90c8acf4c3286c29a5789a2b0131bc2f7",
        "492d0123add0f969cc7f3eeef772c013b06fe1b835b",
    ),
    "test": (
        "b3123445026a7109c894048d592569b09c8344d167e",
        "44127f71236beca2a38125f22debb2c6d92631e4080",
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
