import pytest

from .resnet import resnet20v2_compiled


@pytest.mark.slow
def test_num_parameters_mnist():
    # Prepare
    model = resnet20v2_compiled(input_shape=(32, 32, 3), num_classes=10)
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 574_090
