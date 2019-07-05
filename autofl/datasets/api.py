# pylint: disable=W0611
"""This module makes all methods in the package available which are supposed to be public"""


from .cifar10_random_splits_10 import load_split as cifar10_random_splits_10_load_split
from .cifar10_random_splits_10 import (
    load_splits as cifar10_random_splits_10_load_splits,
)
from .fashion_mnist_10s_600 import load_split as fashion_mnist_10s_600_load_split
from .fashion_mnist_10s_600 import load_splits as fashion_mnist_10s_600_load_splits
