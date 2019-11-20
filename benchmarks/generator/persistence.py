"""
This module stores and retrives datasets from a given storage_dir

A dataset is stored with N>=0 for N=num_splits-1 as
- xN.npy
- yN.npy
- x_test.npy
- y_test.npy
"""
import os
from typing import Dict, List, Optional, Tuple

import numpy as np
from absl import logging

from benchmarks.helpers import storage
from xain.helpers import sha1
from xain.types import FederatedDataset, FnameNDArrayTuple


def save(fname: str, data: np.ndarray, storage_dir: str):
    """Stores an ndarray in storage_dir with as fname

    Args:
        fname (str): Filename
        data: (np.ndarray): Data to store
        storage_dir (str): Absolute path to directory in which data should be stored

    Returns:
        str: SHA1 checksum of stored file
    """
    path = "{}/{}".format(storage_dir, fname)
    np.save(path, data)

    print(f"Saved {path}")

    return sha1.checksum(path)


def dataset_to_fname_ndarray_tuple_list(
    dataset: FederatedDataset
) -> List[Tuple[str, np.ndarray]]:
    """Transforms a FederatedDataset into a list of (filename, ndarray) tuples
    so they can be easily stored on disk.

    Args:
        dataset (FederatedDataset): The dataset to be transformed

    Returns:
        List[Tuple[str, np.ndarray]]
    """
    fname_ndarray_tuples: List[Tuple[str, np.ndarray]] = []
    xy_splits, xy_val, xy_test = dataset

    # Add all splits as tuples to fname_ndarray_tuple
    for i, split in enumerate(xy_splits):
        fname_ndarray_tuples += to_fname_ndarray_tuple(xy=split, suffix=str(i).zfill(2))

    # Add validation set to files which will be stored
    fname_ndarray_tuples += to_fname_ndarray_tuple(xy=xy_val, suffix="val")

    # Add test set to files which will be stored
    fname_ndarray_tuples += to_fname_ndarray_tuple(xy=xy_test, suffix="test")

    return fname_ndarray_tuples


def to_fname_ndarray_tuple(
    suffix: str, xy: Tuple[np.ndarray, np.ndarray]
) -> List[FnameNDArrayTuple]:
    """Given a suffix as well as a tuple of ndarrays a list of (filename, ndarray) tuples
    with length two will be returned.

    Args:
        suffix (str)
        xy (Tuple[np.ndarray, np.ndarray])

    Returns:
        List[FnameNDArrayTuple]

    Example:
        For `to_fname_ndarray_tuple(suffix="val", xy)` xy beeing a tuple of ndarrays the
        result will be a list of length two as::

            [
                ("x_val.npy", xy[0]),
                ("y_val.npy", xy[1]),
            ]
    """
    x, y = xy

    name_x = f"x_{suffix}.npy"
    name_y = f"y_{suffix}.npy"

    return [(name_x, x), (name_y, y)]


def get_dataset_dir(dataset_name: str, local_generator_dir: str) -> str:
    """Will return dataset directory and create it if its not already present

    Args:
        dataset_name (str)
        local_generator_dir (str): Absolut path

    Returns:
        str: Absolut path to dataset_dir
    """
    dataset_dir = os.path.join(local_generator_dir, dataset_name)

    if not os.path.isdir(dataset_dir):
        os.makedirs(dataset_dir)

    return dataset_dir


def save_splits(dataset_name: str, dataset: FederatedDataset, local_generator_dir: str):
    """Saves federated dataset to filesystem

    Args:
        dataset_name (str)
        dataset (FederatedDataset)
        local_generator_dir (str)
    """
    fname_ndarray_tuple = dataset_to_fname_ndarray_tuple_list(dataset)

    dataset_dir = get_dataset_dir(
        dataset_name=dataset_name, local_generator_dir=local_generator_dir
    )

    logging.info("Storing dataset in {}".format(dataset_dir))

    split_hashes: Dict[str, List[Optional[str]]] = {}

    for fname, ndarr in fname_ndarray_tuple:
        sha1cs = save(fname=fname, data=ndarr, storage_dir=dataset_dir)

        storage_key = fname[2:-4]

        if storage_key not in split_hashes:
            split_hashes[storage_key] = [None, None]

        split_hashes[storage_key][0 if "x_" in fname else 1] = sha1cs

    hash_file = os.path.join(dataset_dir, f"../../{dataset_name}.json")
    storage.write_json(split_hashes, hash_file)

    logging.info("{} generated and stored\n".format(dataset_name))
