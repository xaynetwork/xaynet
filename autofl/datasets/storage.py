import hashlib
import os
from typing import Tuple

import numpy
import requests
from absl import flags, logging

from ..types import FederatedDataset

FLAGS = flags.FLAGS


def default_get_local_datasets_dir():
    return FLAGS.local_datasets_dir


def sha1checksum(fpath: str):
    sha1 = hashlib.sha1()

    with open(fpath, "rb") as f:
        while True:
            data = f.read()
            if not data:
                break
            sha1.update(data)

    return sha1.hexdigest()


def get_dataset_dir(dataset_name: str, local_datasets_dir: str) -> str:
    """Will return dataset directory and create it if its not already present"""
    dataset_dir = os.path.join(local_datasets_dir, dataset_name)

    if not os.path.isdir(dataset_dir):
        os.makedirs(dataset_dir)

    return dataset_dir


def fetch_ndarray(url, fpath):
    """Get file from fpath and store at fpath"""
    r = requests.get(url, stream=True)

    logging.info("Fetching file {}".format(url))

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

    Parameters:
    datasets_repository (str): datasets repository base URL
    dataset_name (str): Name of dataset in repository
    ndarray_name (str): ndarray name. Example: "x_00.npy"
    local_datasets_dir (str): Directory in which all local datasets are stored
    cleanup (bool): Cleanup file if it has the wrong hash
    """
    url = "{}/{}/{}".format(FLAGS.datasets_repository, dataset_name, ndarray_name)

    dataset_dir = get_dataset_dir(dataset_name, local_datasets_dir)
    fpath = os.path.join(dataset_dir, ndarray_name)

    if FLAGS.fetch_datasets and not os.path.isfile(fpath):
        fetch_ndarray(url, fpath)

    # Check sha1checksum after conditional fetch even when no download
    # occured and local dataset was used to avoid accidental corruption
    sha1 = sha1checksum(fpath)

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

    ndarray = numpy.load(fpath)

    return ndarray


def load_split(
    dataset_name: str,
    split_id: str,
    split_hashes: Tuple[str, str],
    local_datasets_dir=str,
):
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
    dataset_name: str,
    dataset_split_hashes: dict,
    get_local_datasets_dir=default_get_local_datasets_dir,
) -> FederatedDataset:
    xy_splits = []
    xy_val = (None, None)
    xy_test = (None, None)

    for split_id in dataset_split_hashes:
        data = load_split(
            dataset_name=dataset_name,
            split_id=split_id,
            # passing respective hash tuple for given split_id
            split_hashes=dataset_split_hashes[split_id],
            local_datasets_dir=get_local_datasets_dir(),
        )

        if split_id == "test":
            xy_test = data
        elif split_id == "val":
            xy_val = data
        else:
            xy_splits.append(data)

    return xy_splits, xy_val, xy_test
