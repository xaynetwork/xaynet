"""
This module stores and retrives datasets from a given storage_dir

A dataset is stored with N>=0 for N=num_splits-1 as
- xN.npy
- yN.npy
- x_test.npy
- y_test.npy
"""
import os
from typing import List, Tuple

import numpy as np
from absl import logging

from autofl.types import FederatedDataset, FnameNDArrayTuple


def save(fname: str, data: np.ndarray, storage_dir: str):
    path = "{}/{}".format(storage_dir, fname)
    np.save(path, data)


def dataset_to_fname_ndarray_tuple_list(
    dataset: FederatedDataset
) -> List[Tuple[str, np.ndarray]]:
    fname_ndarray_tuples: List[Tuple[str, np.ndarray]] = []
    xy_splits, xy_test = dataset

    # Add all splits as tuples to fname_ndarray_tuple
    for i, split in enumerate(xy_splits):
        fname_ndarray_tuples += to_fname_ndarray_tuple(xy=split, suffix=str(i).zfill(2))

    # Add test set to files which will be stored
    fname_ndarray_tuples += to_fname_ndarray_tuple(xy=xy_test, suffix="test")

    return fname_ndarray_tuples


def to_fname_ndarray_tuple(
    suffix: str, xy: Tuple[np.ndarray, np.ndarray]
) -> List[FnameNDArrayTuple]:
    x, y = xy

    name_x = "x_{}.npy".format(suffix)
    name_y = "y_{}.npy".format(suffix)

    return [(name_x, x), (name_y, y)]


def get_dataset_dir(dataset_name: str, local_generator_dir: str) -> str:
    """Will return dataset directory and create it if its not already present"""
    dataset_dir = os.path.join(local_generator_dir, dataset_name)

    if not os.path.isdir(dataset_dir):
        os.makedirs(dataset_dir)

    return dataset_dir


def save_splits(dataset_name: str, dataset: FederatedDataset, local_generator_dir: str):
    fname_ndarray_tuple = dataset_to_fname_ndarray_tuple_list(dataset)

    dataset_dir = get_dataset_dir(
        dataset_name=dataset_name, local_generator_dir=local_generator_dir
    )

    logging.info("Storing dataset in {}".format(dataset_dir))

    for fname, ndarr in fname_ndarray_tuple:
        save(fname=fname, data=ndarr, storage_dir=dataset_dir)
