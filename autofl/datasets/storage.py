import os
import shutil

import numpy
import requests

from ..types import FederatedDatasetSplit


def get_dataset_dir(dataset_name: str, local_datasets_dir: str) -> str:
    """Will return dataset directory and create it if its not already present"""
    dataset_dir = os.path.join(local_datasets_dir, dataset_name)

    if not os.path.isdir(dataset_dir):
        os.makedirs(dataset_dir)

    return dataset_dir


def download_remote_ndarray(
    datasets_repository: str,
    dataset_name: str,
    split_name: str,
    local_datasets_dir: str,
) -> FederatedDatasetSplit:
    """Downloads dataset split and loads from disk if already present

    Parameters:
    datasets_repository (str): datasets repository base URL
    dataset_name (str): Name of dataset in repository
    split_name (str): Split name. Example: "x0.npy"
    local_datasets_dir (str): Directory in which all local datasets are stored
    """
    url = "{}/{}/{}".format(datasets_repository, dataset_name, split_name)

    dataset_dir = get_dataset_dir(dataset_name, local_datasets_dir)
    fpath = os.path.join(dataset_dir, split_name)

    if not os.path.isfile(fpath):
        response = requests.get(url, stream=True)

        with open(fpath, "wb") as fin:
            shutil.copyfileobj(response.raw, fin)

    ndarray = numpy.load(fpath)

    return ndarray
