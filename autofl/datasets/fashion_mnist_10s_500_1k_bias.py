from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_10s_500_1k_bias"
DATASET_SPLIT_HASHES = {
    "00": (
        "5a9bb220c8c108496d3631cd511d8fefa8c9fd41",
        "66144f69f6ecee23a49c9aa75ec1e34c60995e88",
    ),
    "01": (
        "b0518ed6e0021dedebf8f2b6abe736bf662a54fa",
        "13b1b03d314d6f6599167b31ac294836a918793d",
    ),
    "02": (
        "e474ed9a428926e079404d426f32fc4b82f1e9bf",
        "45f5814d8215f9c85293dc28360d0f5fc5b111da",
    ),
    "03": (
        "6acb84fcffe56bb04780c823e79a97843e93f654",
        "97bb51bfa4b6c94a323bad3af8af824b946cb77c",
    ),
    "04": (
        "0bc1318ade441b5bf74b9f7a6638c9a851f4e445",
        "a470f9e172655bfe33a0d2fb3335701795ec89fb",
    ),
    "05": (
        "e665ba5eca57acfea316e5e6a68fb72a865110fd",
        "04f33a8c16544841d135d4877409004ee4de924c",
    ),
    "06": (
        "5096e341c94168a3edee9f885761d44f76bfc0d2",
        "8ef416894777deb682460f786975f206610058fd",
    ),
    "07": (
        "2285a523be521f74082d21d76c29ccc8de6e621a",
        "c03dc7315b4cd89a6365a3c488de0669a1c16737",
    ),
    "08": (
        "2879a9bc5ea1273557fefbb3b31cecf70f7f8bbb",
        "7724eeed92157bd12bc7bce5a94e006cbf627175",
    ),
    "09": (
        "b86da7a85bf3d1cb491b58cd0406138e71364a87",
        "6726912e1d14693f79558d87150dee14044139dd",
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
