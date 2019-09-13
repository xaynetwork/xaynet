import numpy as np
import pytest

from xain.benchmark.net import model_fns

from .model_provider import ModelProvider
from .participant import Participant


def test_Participant_x_y_shape_valid():
    # Prepare
    m = None
    x = np.zeros((5, 32, 32, 3), dtype=np.uint8)
    y = np.zeros((5), dtype=np.uint8)
    # Execute
    _ = Participant(0, m, (x, y), (x, y), num_classes=10, batch_size=32)
    # pass if initialization does not raise an exception


def test_Participant_x_y_shape_invalid():
    # Prepare
    m = None
    x = np.zeros((3, 32, 32, 3), dtype=np.uint8)
    y = np.zeros((4), dtype=np.uint8)
    # Execute & assert
    try:
        _ = Participant(0, m, (x, y), (x, y), num_classes=10, batch_size=32)
        pytest.fail("No AssertionError raised")
    except AssertionError:
        pass


def test_Participant_num_examples():
    # Prepare
    num_examples_expected = 19
    num_classes = 10
    model_provider = ModelProvider(model_fns["blog_cnn"])
    x = np.random.randint(
        0, high=256, size=(num_examples_expected, 28, 28, 1), dtype=np.uint8
    )
    y = np.random.randint(
        0, high=num_classes, size=(num_examples_expected), dtype=np.uint8
    )
    np.random.randint
    participant = Participant(
        0, model_provider, (x, y), (x, y), num_classes=num_classes, batch_size=16
    )
    weights = model_provider.init_model().get_weights()

    # Execute
    (_, num_examples_actual), _ = participant.train_round(weights, 2, 0)

    # Assert
    assert num_examples_actual == num_examples_expected


def test_Participant_get_xy_train_volume_by_class():
    # Prepare
    cid_expected = 19
    num_classes = 5
    m = None
    x = np.zeros((4, 32, 32, 3), dtype=np.uint8)
    y = np.array([0, 1, 2, 2], dtype=np.uint8)

    y_volume_by_class_expected = [1, 1, 2, 0, 0]

    # Execute
    p = Participant(
        cid_expected, m, (x, y), (x, y), num_classes=num_classes, batch_size=32
    )

    # Assert
    (cid_actual, y_volume_by_class_actual) = p.metrics()

    assert cid_actual == cid_expected
    assert y_volume_by_class_actual == y_volume_by_class_expected
