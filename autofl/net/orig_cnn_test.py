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
