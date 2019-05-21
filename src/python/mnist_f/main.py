import os
from pprint import pprint
from typing import Any, List, Tuple

import numpy as np
import tensorflow as tf

os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'
tf.logging.set_verbosity(tf.logging.ERROR)


def main():
  x_train, y_train, x_test, y_test = load()
  print("x_train:", x_train.shape) 
  print("y_train:", y_train.shape) 


def load():
  mnist = tf.keras.datasets.mnist
  (x_train, y_train), (x_test, y_test) = mnist.load_data()
  return x_train, y_train, x_test, y_test


if __name__ == "__main__":
  main()
