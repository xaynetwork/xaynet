import concurrent.futures
import os
from typing import Tuple

import numpy as np
import requests
from absl import flags

from xain_fl.helpers.sha1 import checksum
from xain_fl.logger import get_logger
from xain_fl.types import FederatedDataset, Partition

from . import hashes

FLAGS = flags.FLAGS


logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


def default_get_local_datasets_dir():
    """Returns absolute path to datasets directory

    Returns
        str: absolute datasets directory path
    """
    return FLAGS.local_datasets_dir


def get_dataset_dir(dataset_name: str, local_datasets_dir: str) -> str:
    """Will return dataset directory and create it if its not already present

    Args:
        dataset_name (str)
        local_datasets_dir (str)

    Returns:
        str: Absolute directory path to newly created datasets
            directory in local datasets dir
    """
    dataset_dir = os.path.join(local_datasets_dir, dataset_name)

    if not os.path.isdir(dataset_dir):
        os.makedirs(dataset_dir, exist_ok=True)

    return dataset_dir


def fetch_ndarray(url, fpath):
    """Downloads file from url and store at fpath

    Args:
        url (str): URL from which to download the ndarray file
        fpath (str): Absolute path under which to store the file
    """
    r = requests.get(url, stream=True)

    logger.info("Fetching file %s", url)

    if r.status_code != 200:
        raise Exception("Received HTTP Status {} for url {}".format(r.status_code, url))

    handle = open(fpath, "wb")
    for chunk in r.iter_content(chunk_size=1024):
        if chunk:  # filter out keep-alive new chunks
            handle.write(chunk)


def load_ndarray(
    dataset_name: str, ndarray_name: str, ndarray_hash: str, local_datasets_dir: str
):
    """Downloads dataset ndarray and loads from disk if already present

    Args:
        datasets_repository (str): datasets repository base URL
        dataset_name (str): Name of dataset in repository
        ndarray_name (str): ndarray name. Example: "x_00.npy"
        local_datasets_dir (str): Directory in which all local datasets are stored
        cleanup (bool): Cleanup file if it has the wrong hash

    Returns:
        np.ndarray: Loaded numpy ndarray
    """
    url = "{}/{}/{}".format(FLAGS.datasets_repository, dataset_name, ndarray_name)

    dataset_dir = get_dataset_dir(dataset_name, local_datasets_dir)
    fpath = os.path.join(dataset_dir, ndarray_name)

    if FLAGS.fetch_datasets and not os.path.isfile(fpath):
        fetch_ndarray(url, fpath)

    # Check sha1 checksum after conditional fetch even when no download
    # occured and local dataset was used to avoid accidental corruption
    sha1 = checksum(fpath)

    if sha1 != ndarray_hash:
        # Delete the downloaded file if it has the wrong hash
        # Otherwise the next invocation will not download it again
        # which is not a desired behavior
        os.remove(fpath)

        raise Exception(
            "Given hash {} for file {} does not match".format(
                ndarray_hash, ndarray_name
            )
        )

    ndarray = np.load(fpath)

    return ndarray


def load_split(
    dataset_name: str,
    split_id: str,
    split_hashes: Tuple[str, str],
    local_datasets_dir=str,
) -> Partition:
    """Downloads dataset partition and loads from disk if already present

    Args:
        dataset_name (str): Name of dataset in dataset repository
        split_id (str): ndarray name. Example: "x_00.npy"
        split_hashes (str, str): Tuple of hashes of x and y ndarray
        local_datasets_dir (bool): Directory in which all local datasets are stored

    Returns:
        Partition: Federated Dataset Partition
    """
    x_name = "x_{}.npy".format(split_id)
    x_hash = split_hashes[0]

    y_name = "y_{}.npy".format(split_id)
    y_hash = split_hashes[1]

    x = load_ndarray(
        dataset_name=dataset_name,
        ndarray_name=x_name,
        ndarray_hash=x_hash,
        local_datasets_dir=local_datasets_dir,
    )

    y = load_ndarray(
        dataset_name=dataset_name,
        ndarray_name=y_name,
        ndarray_hash=y_hash,
        local_datasets_dir=local_datasets_dir,
    )

    return x, y


def load_splits(
    dataset_name: str, get_local_datasets_dir=default_get_local_datasets_dir
) -> FederatedDataset:
    """Loads FederatedDataset from local datasets directory and from remote
    datasets repository if not locally present

    Args:
        dataset_name (str): Name of dataset in repository
        get_local_datasets_dir (Callable): Function which returns an absolute
            path to datasets directory

    Returns:
        FederatedDataset
    """

    xy_splits = []
    xy_val = (None, None)
    xy_test = (None, None)

    dataset_split_hashes = hashes.datasets[dataset_name]

    local_datasets_dir = get_local_datasets_dir()

    def load_method(split_id: str):
        data = load_split(
            dataset_name=dataset_name,
            split_id=split_id,
            # passing respective hash tuple for given split_id
            split_hashes=dataset_split_hashes[split_id],
            local_datasets_dir=local_datasets_dir,
        )

        return split_id, data

    with concurrent.futures.ThreadPoolExecutor() as executor:
        future_results = [
            executor.submit(load_method, split_id) for split_id in dataset_split_hashes
        ]
        concurrent.futures.wait(future_results)

    for future in future_results:
        split_id, data = future.result()

        if split_id == "test":
            xy_test = data
        elif split_id == "val":
            xy_val = data
        else:
            xy_splits.append(data)

    return xy_splits, xy_val, xy_test
