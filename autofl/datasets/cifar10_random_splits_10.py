from ..types import FederatedDataset
from . import storage
from .config import get_config

DATASET_NAME = "cifar10_random_splits_10"

DATASET_SPLIT_HASHES = {
    "x_00.npy": "d302de44a32a31a6c2930b053684ecd8b42cb681",
    "x_01.npy": "c15f47ee6952558bc679866a49dbf9671d1c3de0",
    "x_02.npy": "3d1043525d1661f9867fcdfd885d74877154d1c6",
    "x_03.npy": "36cd5d81f399a94251da0230c7dce6dfbcde08ca",
    "x_04.npy": "bbdf9c8e8fb3caff23016a22b9b7fd823a79b96e",
    "x_05.npy": "ab028c7e44c8e1786aa9ca13a463bbd2219ddb7d",
    "x_06.npy": "b4e5e7360682f3708cffba852ba8fb819cfa919c",
    "x_07.npy": "ab24961a27c02e509d3589737b42f44be85b4b59",
    "x_08.npy": "277c169c7a71e78a2c726e6d7558b614778f2036",
    "x_09.npy": "e0067be90c8acf4c3286c29a5789a2b0131bc2f7",
    "y_00.npy": "e3f704ced3290d35cf4b2b5b59a04267ddb2c985",
    "y_01.npy": "e5ad82230e725e7c130ea30f968b5c5827069005",
    "y_02.npy": "17aecfe65967a3af1e1e56850a46f30cf994c7a2",
    "y_03.npy": "3ff2eaaa878fdabdec881a8c56ee359b41a5058d",
    "y_04.npy": "e3217dbbacb655b008d82f21dfe7ac0fd77a5e42",
    "y_05.npy": "8e85b62f14cc3ea8562d6de225caf2e5621e6871",
    "y_06.npy": "a5c7433c5ec57f8a79704c6cc5bcb63dfd733b5c",
    "y_07.npy": "7e2e41829d7fd0e72ba9824ded5fcc0fa51ae671",
    "y_08.npy": "2fa72d6d81bb2713bd2c50430b063b38c8b0bf07",
    "y_09.npy": "492d0add0f969cc7f3eeef772c013b06fe1b835b",
    "x_test.npy": "b3445026a7109c894048d592569b09c8344d167e",
    "y_test.npy": "44127f76beca2a38125f22debb2c6d92631e4080",
}


def load_splits() -> FederatedDataset:
    xy_splits = []

    for i in range(10):
        split_id = str(i).zfill(2)
        xy_splits.append(load_split(split_id))

    xy_test = load_split("test")

    return xy_splits, xy_test


def load_split(split_id, local_datasets_dir: str = get_config("local_datasets_dir")):
    valid_ids = set(
        ["00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "test"]
    )

    assert split_id in valid_ids

    x_split_name = "x_{}.npy".format(split_id)
    y_split_name = "y_{}.npy".format(split_id)

    x_i = storage.download_remote_ndarray(
        datasets_repository=get_config("datasets_repository"),
        dataset_name=DATASET_NAME,
        split_name=x_split_name,
        local_datasets_dir=local_datasets_dir,
    )

    y_i = storage.download_remote_ndarray(
        datasets_repository=get_config("datasets_repository"),
        dataset_name=DATASET_NAME,
        split_name=y_split_name,
        local_datasets_dir=local_datasets_dir,
    )

    return (x_i, y_i)
