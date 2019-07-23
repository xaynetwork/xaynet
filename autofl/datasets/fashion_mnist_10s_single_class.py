from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_single_class"
DATASET_SPLIT_HASHES = {
    "00": (
        "396fde45d36c39d9348717aa022a84fd327ae40d",
        "2c1b9cccdbea1d5aa0bedd5664be75f7ce99869d",
    ),
    "01": (
        "5383a9f5a40cca30ecadfb054e9e4ac9b9785376",
        "0a5de67d329bd8389bd2b0ed5407e09e00c6cd90",
    ),
    "02": (
        "c2e5b56f4ed507ae260d87e7128431566e71c6bc",
        "4bdb25616ddf967576645ef9e4f18af29a73a704",
    ),
    "03": (
        "0d366ad481ee7db063f75df310c8e9c598d30989",
        "997a507a09282b4c8e48304ac55670604d867716",
    ),
    "04": (
        "d768d839ab672a6f937c013abbc18b1c3c5eba11",
        "97b2c1fae97aea5d07b6765ae0ac48f0b4b6129e",
    ),
    "05": (
        "61a121bb680b86def87349d794742541f12ebba1",
        "6709578defcc510e86591030196c99aa64bac081",
    ),
    "06": (
        "bb8169c99ce096807a6ea802fa293528e33a13f8",
        "636b8027f35a807847e0112c1767d845ea318636",
    ),
    "07": (
        "30c410e07bac2d456ad2604f44cb63e4ebd32c80",
        "d2f2d9e5d588f4e48b2778281203f2e54fed5688",
    ),
    "08": (
        "b9bad150a1af3c94bf7316b0367722e07e7a185f",
        "5fca148c17cfb7e2df2b54f458104d9bd3eb022e",
    ),
    "09": (
        "f65015a87915591384383e94ccf3938c797d2fe4",
        "6cfefa4b1930b4066ab0d4b83d1cedcf5280e003",
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
