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

from .typing import FederatedDataset, FilenameNDArrayTuple


def save(filename: str, data: np.ndarray, storage_dir: str):
    path = "{}/{}".format(storage_dir, filename)
    np.save(path, data)


def load(filename: str, storage_dir: str) -> np.ndarray:
    path = "{}/{}".format(storage_dir, filename)
    return np.load(path)


def dataset_to_filename_ndarray_tuple_list(
    dataset: FederatedDataset
) -> List[Tuple[str, np.ndarray]]:
    filename_ndarray_tuples: List[Tuple[str, np.ndarray]] = []
    xy_splits, xy_test = dataset

    # Add all splits as tuples to filename_ndarray_tuple
    for i, split in enumerate(xy_splits):
        filename_ndarray_tuples += generate_filename_ndarray_tuple(
            xy=split, suffix=str(i)
        )

    # Add test set to files which will be stored
    filename_ndarray_tuples += generate_filename_ndarray_tuple(
        xy=xy_test, suffix="_test"
    )

    return filename_ndarray_tuples


def generate_filename_ndarray_tuple(
    suffix: str, xy: Tuple[np.ndarray, np.ndarray]
) -> List[FilenameNDArrayTuple]:
    x, y = xy

    name_x = "x{}.npy".format(suffix)
    name_y = "y{}.npy".format(suffix)

    return [(name_x, x), (name_y, y)]


def strip_npy_ending(fn):
    return fn[:-4]


def generate_dataset_from_filename_ndarray_tuples(
    tuples: List[FilenameNDArrayTuple]
) -> FederatedDataset:
    # Get highest index from xN
    tuples = [(strip_npy_ending(fn), nda) for fn, nda in tuples]
    max_index = [int(fn[1:]) for fn, _ in tuples if "test" not in fn]
    count_tuples = max(max_index) + 1

    xy_splits_list = [[None, None]] * count_tuples
    xy_test_list = [None, None]

    for fn, nda in tuples:
        if "_test" not in fn:
            index = int(fn[1:])

            if "x" in fn:
                xy_splits_list[index][0] = nda

            if "y" in fn:
                xy_splits_list[index][1] = nda

        if fn == "x_test":
            xy_test_list[0] = nda

        if fn == "y_test":
            xy_test_list[1] = nda

    # Change List[List[ndarray, ndarray]] into List[Tuple[ndarray, ndarray]]
    xy_splits = [(xy[0], xy[1]) for xy in xy_splits_list]

    # Change xy_test to a tuple
    xy_test: Tuple[np.ndarray, np.ndarray] = (xy_test_list[0], xy_test_list[1])

    return (xy_splits, xy_test)


def save_splits(dataset: FederatedDataset, storage_dir: str):
    logging.info("Storing dataset in {}".format(storage_dir))

    filename_ndarray_tuple = dataset_to_filename_ndarray_tuple_list(dataset)

    for filename, ndarr in filename_ndarray_tuple:
        save(filename=filename, data=ndarr, storage_dir=storage_dir)


def load_splits(storage_dir: str):
    logging.info("Retrieving dataset from {}".format(storage_dir))

    files = list_files_for_dataset(storage_dir)

    # Load data from disk
    filename_ndarray_tuples = [(fn, load(fn, storage_dir)) for fn in files]

    # generate dataset from tuples
    dataset = generate_dataset_from_filename_ndarray_tuples(filename_ndarray_tuples)

    return dataset


def is_npy_file(fn):
    return fn[-4:] == ".npy"


def list_files_for_dataset(storage_dir: str) -> List[str]:
    files = os.listdir(storage_dir)
    return list(filter(is_npy_file, files))
