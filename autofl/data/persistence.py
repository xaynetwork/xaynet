"""
This module stores and retrives datasets from a given storage_dir

A dataset is stored with N>=0 for N=num_splits-1 as
- xN.npy
- yN.npy
- x_test.npy
- y_test.npy
"""
import os
from typing import List, Optional, Set, Tuple

import numpy as np
from absl import logging

from autofl.types import FederatedDataset, FnameNDArrayTuple


def save(fname: str, data: np.ndarray, storage_dir: str):
    path = "{}/{}".format(storage_dir, fname)
    np.save(path, data)


def load(fname: str, storage_dir: str) -> np.ndarray:
    path = "{}/{}".format(storage_dir, fname)
    return np.load(path)


def dataset_to_fname_ndarray_tuple_list(
    dataset: FederatedDataset
) -> List[Tuple[str, np.ndarray]]:
    fname_ndarray_tuples: List[Tuple[str, np.ndarray]] = []
    xy_splits, xy_test = dataset

    # Add all splits as tuples to fname_ndarray_tuple
    for i, split in enumerate(xy_splits):
        fname_ndarray_tuples += to_fname_ndarray_tuple(xy=split, suffix=str(i))

    # Add test set to files which will be stored
    fname_ndarray_tuples += to_fname_ndarray_tuple(xy=xy_test, suffix="_test")

    return fname_ndarray_tuples


def to_fname_ndarray_tuple(
    suffix: str, xy: Tuple[np.ndarray, np.ndarray]
) -> List[FnameNDArrayTuple]:
    x, y = xy

    name_x = "x{}.npy".format(suffix)
    name_y = "y{}.npy".format(suffix)

    return [(name_x, x), (name_y, y)]


def strip_npy_ending(fn):
    return fn[:-4]


def dataset_from_fname_ndarray_tuples(
    tuples: List[FnameNDArrayTuple]
) -> FederatedDataset:
    # Get highest index from xN
    tuples = [(strip_npy_ending(fname), nda) for fname, nda in tuples]
    max_index = [int(fname[1:]) for fname, _ in tuples if "test" not in fname]
    count_tuples = max(max_index) + 1

    xy_splits_list = [[None, None]] * count_tuples
    xy_test_list = [None, None]

    for fname, nda in tuples:
        if "_test" not in fname:
            index = int(fname[1:])

            if "x" in fname:
                xy_splits_list[index][0] = nda

            if "y" in fname:
                xy_splits_list[index][1] = nda

        if fname == "x_test":
            xy_test_list[0] = nda

        if fname == "y_test":
            xy_test_list[1] = nda

    # Change List[List[ndarray, ndarray]] into List[Tuple[ndarray, ndarray]]
    xy_splits = [(xy[0], xy[1]) for xy in xy_splits_list]

    # Change xy_test to a tuple
    xy_test: Tuple[np.ndarray, np.ndarray] = (xy_test_list[0], xy_test_list[1])

    return (xy_splits, xy_test)


def save_splits(dataset: FederatedDataset, storage_dir: str):
    logging.info("Storing dataset in {}".format(storage_dir))

    fname_ndarray_tuple = dataset_to_fname_ndarray_tuple_list(dataset)

    for fname, ndarr in fname_ndarray_tuple:
        save(fname=fname, data=ndarr, storage_dir=storage_dir)


def load_splits(storage_dir: str) -> FederatedDataset:
    logging.info("Retrieving dataset from {}".format(storage_dir))

    files = list_files_for_dataset(storage_dir)

    # Load data from disk
    fname_ndarray_tuples = [(fname, load(fname, storage_dir)) for fname in files]

    # Create dataset from tuples
    dataset = dataset_from_fname_ndarray_tuples(fname_ndarray_tuples)

    return dataset


def load_local_dataset(
    dataset_name: str, local_datasets_dir: str
) -> Optional[FederatedDataset]:
    # Check if dataset exists locally and if so load and return
    dataset_dir = os.path.join(local_datasets_dir, dataset_name)

    if dataset_name in list_datasets(local_datasets_dir):
        return load_splits(storage_dir=dataset_dir)

    return None


def is_npy_file(fname):
    return fname.split(".")[-1] == "npy"


def list_files_for_dataset(storage_dir: str) -> List[str]:
    files = os.listdir(storage_dir)
    return list(filter(is_npy_file, files))


def list_datasets(local_datasets_dir: str) -> Set[str]:
    files = os.listdir(local_datasets_dir)

    directories = []

    for fname in files:
        full_path = os.path.join(local_datasets_dir, fname)
        if os.path.isdir(full_path):
            directories.append(fname)

    return set(directories)
