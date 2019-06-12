import pytest
import tensorflow as tf


@pytest.fixture
def dataset_mnist():
    (x_train, y_train), (x_test, y_test) = tf.keras.datasets.mnist.load_data()
    return (x_train, y_train, x_test, y_test)


@pytest.fixture
def dataset_cifar10():
    (x_train, y_train), (x_test, y_test) = tf.keras.datasets.cifar10.load_data()
    return (x_train, y_train, x_test, y_test)
