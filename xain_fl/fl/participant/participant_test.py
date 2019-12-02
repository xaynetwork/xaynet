import numpy as np
import pytest

# from benchmarks.benchmark.net import model_fns  # FIXME refactor
from xain_fl.datasets import load_splits

# from .model_provider import ModelProvider  # FIXME refactor
from .participant import Participant, _xy_train_volume_by_class


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


# FIXME refactor to remove dependency on benchmark code
# def test_Participant_num_examples():
#     # Prepare
#     num_examples_expected = 19
#     num_classes = 10
#     model_provider = ModelProvider(model_fns["blog_cnn"], lr_fn_fn=None)
#     x = np.random.randint(
#         0, high=256, size=(num_examples_expected, 28, 28, 1), dtype=np.uint8
#     )
#     y = np.random.randint(
#         0, high=num_classes, size=(num_examples_expected), dtype=np.uint8
#     )
#     participant = Participant(
#         0,
#         model_provider,
#         (x, y),
#         (x, y),
#         num_classes=num_classes,
#         batch_size=16,
#         use_lr_fn=False,
#     )
#     weights = model_provider.init_model().get_weights()
#
#     # Execute
#     (_, num_examples_actual), _, _ = participant.train_round(weights, 2, 0)
#
#     # Assert
#     assert num_examples_actual == num_examples_expected


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


@pytest.mark.parametrize(
    "num_classes_total, num_classes_in_partition", [(4, 1), (7, 5), (10, 10)]
)
def test_xy_train_volume_by_class(num_classes_total, num_classes_in_partition):
    # Prepare
    y_train = np.arange(num_classes_in_partition, dtype=np.int8)
    x_train = np.ones((y_train.size))  # not relevant; only needed to avoid type errors
    xy_train = (x_train, y_train)

    # Execute
    result = _xy_train_volume_by_class(num_classes=num_classes_total, xy_train=xy_train)

    # Assert
    assert len(result) == num_classes_total
    if num_classes_total == num_classes_in_partition:
        # As each class is equal times present the set should contain only one element
        assert set(result) == {1}
    else:
        # As each class is equal or zero times present the set should contain 1 and 0
        assert set(result) == {0, 1}


@pytest.mark.slow
@pytest.mark.integration
def test_xy_train_volume_by_class_with_federated_dataset():
    # Prepare
    dataset_name = "fashion-mnist-100p-b1_045"
    xy_partitions, _, _ = load_splits(dataset_name)
    num_examples_expected = sum([x.shape[0] for x, _ in xy_partitions])

    # We need to find out which classes are present in our dataset
    # (actually we know it but making it a bit more adaptable in case we parameterize it)
    all_classes = set()

    for _, y_train in xy_partitions:
        classes = np.unique(y_train)
        for c in classes:
            all_classes.add(c)

    num_classes_total = len(all_classes)

    results = []

    # Execute
    for xy_train in xy_partitions:
        _, y_train = xy_train
        r = _xy_train_volume_by_class(num_classes=num_classes_total, xy_train=xy_train)
        results.append(r)

    # Assert
    num_examples_actual = 0

    for r in results:
        num_examples_actual += sum(r)

    assert num_examples_expected == num_examples_actual
