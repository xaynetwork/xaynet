import os
from pprint import pprint
from typing import Any
from typing import List
from typing import Optional
from typing import Tuple

import numpy as np
import tensorflow as tf

os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'
tf.logging.set_verbosity(tf.logging.ERROR)

PARTITIONS = 3


def main():
  # Load dataset
  x_train, y_train, x_test, y_test = load()
  print("Training set before split:")
  print("\tx_train:", x_train.shape, type(x_train))
  print("\ty_train:", y_train.shape, type(x_train))

  # Shuffle both x and y with the same permutation
  x_train, y_train = shuffle(x_train, y_train)
  print("Training set after shuffle:")
  print("\tx_train:", x_train.shape, type(x_train))
  print("\ty_train:", y_train.shape, type(x_train))

  x_splits, y_splits = split(x_train, y_train, PARTITIONS)
  x_train_0 = x_splits[0]
  y_train_0 = y_splits[0]
  print("Training set after split:")
  for i, (x_split, y_split) in enumerate(zip(x_splits, y_splits)):
    print("\t", str(i), "x_split:", x_split.shape, type(x_split))
    print("\t", str(i), "y_split:", y_split.shape, type(y_split))


def load() -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
  mnist = tf.keras.datasets.mnist
  (x_train, y_train), (x_test, y_test) = mnist.load_data()
  return x_train, y_train, x_test, y_test


def shuffle(x: np.ndarray, y: np.ndarray, seed: Optional[int] = None) -> Tuple[np.ndarray, np.ndarray]:
  assert x.shape[0] == y.shape[0]
  p = np.random.RandomState(seed=seed).permutation(x.shape[0])
  x_shuffled = x[p]
  y_shuffled = y[p]
  return x_shuffled, y_shuffled


def split(x: np.ndarray, y: np.ndarray, num_splits: int) -> Tuple[List[Any], List[Any]]:
  x_splits = np.split(x, indices_or_sections=num_splits, axis=0)
  y_splits = np.split(y, indices_or_sections=num_splits, axis=0)
  return x_splits, y_splits


if __name__ == "__main__":
  main()
