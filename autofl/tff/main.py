import os
from pprint import pprint
from typing import Any
from typing import List
from typing import Optional
from typing import Tuple

import numpy as np
import tensorflow as tf
import tensorflow_federated as tff

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.logging.set_verbosity(tf.logging.ERROR)

PARTITIONS = 3


def main():
    tf.compat.v1.enable_v2_behavior()

    tff.federated_computation(lambda: "Hello, World!")()
    emnist_train, emnist_test = tff.simulation.datasets.emnist.load_data()
    l = len(emnist_train.client_ids)
    print(l)


if __name__ == "__main__":
    main()
