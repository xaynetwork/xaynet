import os
from typing import List, Tuple

import numpy as np
from absl import logging

from .config import get_config
from .typing import FederatedDataset


def save(
    filename: str, data: np.ndarray, storage_dir: str = get_config("local_dataset_dir")
):
    path = "{}/{}".format(storage_dir, filename)
    np.save(path, data)


def load(
    filename: str, storage_dir: str = get_config("local_dataset_dir")
) -> np.ndarray:
    path = "{}/{}".format(storage_dir, filename)
    return np.load(path)


def dataset_to_filename_ndarray_tuple(
    filename_template: str, dataset: FederatedDataset
) -> List[Tuple[str, np.ndarray]]:
    filename_ndarray_tuples: List[Tuple[str, np.ndarray]] = []
    xy_splits, xy_test = dataset

    # add all splits as tuples to filename_ndarray_tuple
    for i, split in enumerate(xy_splits):
        filename_ndarray_tuples += generate_filename_ndarray_tuple(
            filename_template, str(i), split
        )

    # add test set to files which will be stored
    filename_ndarray_tuples += generate_filename_ndarray_tuple(
        filename_template, "_test", xy_test
    )

    return filename_ndarray_tuples


def generate_filename_ndarray_tuple(
    filename_template: str, suffix: str, xy: Tuple[np.ndarray, np.ndarray]
) -> List[Tuple[str, np.ndarray]]:
    x, y = xy
    filename_ndarray_tuple = [
        (filename_template.format("x" + suffix), x),
        (filename_template.format("y" + suffix), y),
    ]
    return filename_ndarray_tuple


def save_splits(
    filename_template: str,
    dataset: FederatedDataset,
    storage_dir: str = get_config("local_dataset_dir"),
):
    filename_ndarray_tuple = dataset_to_filename_ndarray_tuple(
        filename_template, dataset
    )
    for filename, ndarr in filename_ndarray_tuple:
        save(filename=filename, data=ndarr, storage_dir=storage_dir)


def list_files_for_template(
    storage_dir: str = get_config("local_dataset_dir")
) -> List[str]:
    files_in_dataset_dir = os.listdir(storage_dir)

    return files_in_dataset_dir


def load_splits(
    filename_template: str, storage_dir: str = get_config("local_dataset_dir")
) -> FederatedDataset:
    """loads a dataset given a filename_template from storage_dir

    Args:
        filename_template (str): A filename template of the form `*_NUM_SPLITS_{}.npy`
        storage_dir (str): The full path of the directory in which the dataset is stored
    """
    files_in_dataset_dir = os.listdir(storage_dir)

    logging.debug(filename_template)
    logging.debug(files_in_dataset_dir)

    return np.ndarray([])
