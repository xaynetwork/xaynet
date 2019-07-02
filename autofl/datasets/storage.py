import hashlib
import os
import shutil
from typing import Tuple

import numpy
import requests


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
    response = requests.get(url, stream=True)

    with open(fpath, "wb") as fin:
        shutil.copyfileobj(response.raw, fin)


def load_ndarray(
    datasets_repository: str,
    dataset_name: str,
    ndarray_name: str,
    ndarray_hash: str,
    local_datasets_dir: str,
):
    """Downloads dataset ndarray and loads from disk if already present

    Parameters:
    datasets_repository (str): datasets repository base URL
    dataset_name (str): Name of dataset in repository
    ndarray_name (str): ndarray name. Example: "x0.npy"
    local_datasets_dir (str): Directory in which all local datasets are stored
    """
    url = "{}/{}/{}".format(datasets_repository, dataset_name, ndarray_name)

    dataset_dir = get_dataset_dir(dataset_name, local_datasets_dir)
    fpath = os.path.join(dataset_dir, ndarray_name)

    if not os.path.isfile(fpath):
        fetch_ndarray(url, fpath)

    sha1 = sha1checksum(fpath)

    assert sha1 == ndarray_hash, "Given hash does not match file hash"

    ndarray = numpy.load(fpath)

    return ndarray


def load_split(
    datasets_repository: str,
    dataset_name: str,
    split_id: str,
    split_hashes: Tuple[str, str],
    local_datasets_dir=str,
):
    x_name = "x_{}.ndy".format(split_id)
    x_hash = split_hashes[0]

    y_name = "y_{}.ndy".format(split_id)
    y_hash = split_hashes[1]

    x = load_ndarray(
        datasets_repository=datasets_repository,
        dataset_name=dataset_name,
        ndarray_name=x_name,
        ndarray_hash=x_hash,
        local_datasets_dir=local_datasets_dir,
    )

    y = load_ndarray(
        datasets_repository=datasets_repository,
        dataset_name=dataset_name,
        ndarray_name=y_name,
        ndarray_hash=y_hash,
        local_datasets_dir=local_datasets_dir,
    )

    return x, y
