import random

import numpy as np
import tensorflow as tf

from .orig_cnn import orig_cnn_compiled


def test_num_parameters_mnist():
    # Prepare
    model = orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10)
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 582_026


def test_num_parameters_cifar():
    # Prepare
    model = orig_cnn_compiled(input_shape=(32, 32, 3), num_classes=10)
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 878_538


def test_seed_mnist():
    # Prepare
    random.seed(0)
    np.random.seed(1)
    tf.set_random_seed(2)
    MODEL_SEED = 3

    # Execute
    model_a = orig_cnn_compiled(
        input_shape=(28, 28, 1), num_classes=10, seed=MODEL_SEED
    )
    model_b = orig_cnn_compiled(
        input_shape=(28, 28, 1), num_classes=10, seed=MODEL_SEED
    )

    # Assert
    assert model_a.count_params() == model_b.count_params()
    # Ensure weight matrices are exactly the same
    ws_a = model_a.get_weights()
    ws_b = model_b.get_weights()
    # pylint: disable-msg=consider-using-enumerate
    for layer_i in range(len(ws_a)):
        # pylint: disable-msg=consider-using-enumerate
        for weight_i in range(len(ws_a[layer_i])):
            w_a = ws_a[layer_i][weight_i]
            w_b = ws_b[layer_i][weight_i]
            np.testing.assert_equal(w_a, w_b)


def test_seed_cifar():
    # Prepare
    random.seed(0)
    np.random.seed(1)
    tf.set_random_seed(2)
    MODEL_SEED = 3

    # Execute
    model_a = orig_cnn_compiled(
        input_shape=(32, 32, 3), num_classes=10, seed=MODEL_SEED
    )
    model_b = orig_cnn_compiled(
        input_shape=(32, 32, 3), num_classes=10, seed=MODEL_SEED
    )

    # Assert
    assert model_a.count_params() == model_b.count_params()
    # Ensure weight matrices are exactly the same
    ws_a = model_a.get_weights()
    ws_b = model_b.get_weights()
    # pylint: disable-msg=consider-using-enumerate
    for layer_i in range(len(ws_a)):
        # pylint: disable-msg=consider-using-enumerate
        for weight_i in range(len(ws_a[layer_i])):
            w_a = ws_a[layer_i][weight_i]
            w_b = ws_b[layer_i][weight_i]
            np.testing.assert_equal(w_a, w_b)
