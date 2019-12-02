from typing import Tuple

import numpy as np
from tensorflow.data import Dataset

from xain_fl.types import FederatedDataset, KerasDataset


def load(keras_dataset: Dataset) -> KerasDataset:
    """Loads Keras Dataset in predictable form

    Args:
        keras_dataset (Dataset)

    Returns:
        Dataset
    """
    (x_train, y_train), (x_test, y_test) = keras_dataset.load_data()

    y_train = y_train.reshape((y_train.shape[0],))
    y_test = y_test.reshape((y_test.shape[0],))

    return (x_train, y_train), (x_test, y_test)


def unpack_keras_and_federated_dataset(
    keras_dataset: Dataset, federated_dataset: FederatedDataset
) -> Tuple[Tuple[np.ndarray, np.ndarray], Tuple[np.ndarray, np.ndarray]]:
    """Unpacks Keras Dataset and Federated Dataset

    Args:
        keras_dataset (Dataset)
        federated_dataset (FederatedDataset)

    Returns:
        (np.ndarray, np.ndarray), (np.ndarray, np.ndarray)
    """
    # Unpack and group each dataset
    (x1_keras, y1_keras), (x2_keras, y2_keras) = keras_dataset
    xy1_fed_splits, (x2_fed, y2_fed), (x3_fed, y3_fed) = federated_dataset
    x1_fed_splits, y1_fed_splits = zip(*xy1_fed_splits)

    x_keras = np.concatenate([x1_keras, x2_keras], axis=0)
    y_keras = np.concatenate([y1_keras, y2_keras], axis=0)
    assert x_keras.shape[0] == y_keras.shape[0]

    x_fed = np.concatenate([*x1_fed_splits, x2_fed, x3_fed])
    y_fed = np.concatenate([*y1_fed_splits, y2_fed, y3_fed])
    assert x_fed.shape[0] == y_fed.shape[0]

    # Shapes of unpacked and grouped datasets should be same
    assert x_keras.shape == x_fed.shape
    assert y_keras.shape == y_fed.shape

    return (x_keras, y_keras), (x_fed, y_fed)


def hash_xy(x, y):
    """Produces unique hash for given x and y

    Args:
        x (np.ndarray)
        y (np.ndarray)

    Returns
        str: Hash of combination of x and y
    """
    return hash(x.tobytes() + y.tobytes())


def assert_dataset_origin(keras_dataset, federated_dataset):
    """Asserts that given Federated Dataset is derived from given
    Keras Dataset. Function will throw if otherwise.

    Args:
        keras_dataset (Dataset)
        federated_dataset (FederatedDataset)
    """
    # Unpack and group each dataset
    (x_keras, y_keras), (x_fed, y_fed) = unpack_keras_and_federated_dataset(
        keras_dataset, federated_dataset
    )

    assert x_keras.shape == x_fed.shape
    assert y_keras.shape == y_fed.shape

    hash_table = {hash_xy(x, y): 0 for (x, y) in zip(x_keras, y_keras)}

    for (x, y) in zip(x_fed, y_fed):
        hash_table[hash_xy(x, y)] += 1

    counts = list(hash_table.values())
    unq = np.unique(counts)

    assert len(unq) == 1, "Some examples are duplicate or not existing in keras dataset"
    assert unq[0] == 1, "Federated example not found in original keras dataset"
