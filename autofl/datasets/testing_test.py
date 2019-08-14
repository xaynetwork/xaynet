import pytest

from . import testing


def test_assert_dataset_origin(
    mock_simple_keras_dataset, mock_simple_federated_dataset
):
    # Execute & Assert
    testing.assert_dataset_origin(
        keras_dataset=mock_simple_keras_dataset,
        federated_dataset=mock_simple_federated_dataset,
    )


def test_assert_dataset_origin_raise(
    mock_simple_keras_dataset, mock_simple_federated_dataset
):
    # Prepare
    # Get reference to x_train
    (x_train, _), _ = mock_simple_keras_dataset

    # And change one number in the first colum of the first example
    x_train[0][0][0] += 1

    # Execute & Assert
    with pytest.raises(Exception):
        testing.assert_dataset_origin(
            keras_dataset=mock_simple_keras_dataset,
            federated_dataset=mock_simple_federated_dataset,
        )
