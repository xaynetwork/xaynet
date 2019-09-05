import random

import numpy as np
import pytest
import tensorflow as tf

from .orig_cnn import orig_cnn_compiled


@pytest.mark.slow
def test_num_parameters_mnist():
    # Prepare
    model = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10)
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 1_663_370


@pytest.mark.slow
def test_num_parameters_cifar():
    # Prepare
    model = orig_cnn_compiled(input_shape=(32, 32, 3), num_classes=10)
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 2_156_490


@pytest.mark.slow
def test_seed_mnist():
    # Prepare
    random.seed(0)
    np.random.seed(1)
    tf.set_random_seed(2)
    SEED = 3

    # Execute
    model_a = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10, seed=SEED)
    model_b = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10, seed=SEED)

    # Assert
    assert model_a.count_params() == model_b.count_params()
    # Ensure weight matrices are exactly the same
    ws_a = model_a.get_weights()
    ws_b = model_b.get_weights()
    # pylint: disable-msg=consider-using-enumerate
    for layer_i in range(len(ws_a)):
        w_a = ws_a[layer_i]
        w_b = ws_b[layer_i]
        assert w_a.shape == w_b.shape
        np.testing.assert_equal(w_a, w_b)


@pytest.mark.slow
def test_seed_cifar():
    # Prepare
    random.seed(0)
    np.random.seed(1)
    tf.set_random_seed(2)
    SEED = 3

    # Execute
    model_a = orig_cnn_compiled(input_shape=(32, 32, 3), num_classes=10, seed=SEED)
    model_b = orig_cnn_compiled(input_shape=(32, 32, 3), num_classes=10, seed=SEED)

    # Assert
    assert model_a.count_params() == model_b.count_params()
    # Ensure weight matrices are exactly the same
    ws_a = model_a.get_weights()
    ws_b = model_b.get_weights()
    # pylint: disable-msg=consider-using-enumerate
    for layer_i in range(len(ws_a)):
        w_a = ws_a[layer_i]
        w_b = ws_b[layer_i]
        assert w_a.shape == w_b.shape
        np.testing.assert_equal(w_a, w_b)


@pytest.mark.slow
def test_seed_unequal():
    # Prepare
    random.seed(0)
    np.random.seed(1)
    tf.set_random_seed(2)

    # Execute
    model_a = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10, seed=3)
    model_b = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10, seed=4)

    # Assert
    assert model_a.count_params() == model_b.count_params()
    # Ensure weight matrices are exactly the same
    ws_a = model_a.get_weights()
    ws_b = model_b.get_weights()
    # pylint: disable-msg=consider-using-enumerate
    for layer_i in range(len(ws_a)):
        assert ws_a[layer_i].shape == ws_b[layer_i].shape
        if ws_a[layer_i].ndim == 1:
            continue  # Bias can be the same
        w_a = ws_a[layer_i]
        w_b = ws_b[layer_i]
        assert not np.any(np.equal(w_a, w_b))
