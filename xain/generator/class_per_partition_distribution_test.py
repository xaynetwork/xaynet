import numpy as np
import pytest

from .class_per_partition_distribution import distribution


@pytest.mark.parametrize("cpp", [1, 5, 10])
def test_distribution(cpp):
    # Prepare
    num_partitions = 10
    num_classes = 10
    num_sections = cpp * num_partitions
    num_per_class = num_sections / num_classes

    # Execute
    p_dist = distribution(
        num_classes=num_classes, num_partitions=num_partitions, cpp=cpp
    )

    # Assert
    assert len(p_dist) == num_partitions

    for partition in p_dist:
        non_zero = np.count_nonzero(partition)
        assert non_zero == cpp

    assert np.sum(p_dist) == num_sections

    # Each class should occur globally same number of times
    for column in p_dist.T:
        assert np.sum(column) == num_per_class
