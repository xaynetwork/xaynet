import numpy as np
import pytest

from . import volume_distributions


def test_fashion_mnist_distributions():
    # Prepare
    expected_sum = 54_000

    # Execute
    dists = volume_distributions.fashion_mnist_100p()

    # Assert
    assert len(dists) == 10
    for dist in dists:
        assert sum(dist[1]) == expected_sum


def test_cifar_10_distributions():
    # Prepare
    expected_sum = 45_000

    # Execute
    dists = volume_distributions.cifar_10_100p()

    # Assert
    assert len(dists) == 10
    for dist in dists:
        assert sum(dist[1]) == expected_sum


@pytest.mark.parametrize(
    "b, expected, target",
    [
        (1.0, 540.0, 54_000),
        (1.005, 417.94000000000017, 54_000),
        (1.01, 317.03099999999995, 54_000),
        (1.0, 450.0, 45_000),
        (1.005, 348.351, 45_000),
        (1.01, 264.28, 45_000),
    ],
)
def test_brute_force_a(b, expected, target):
    # Execute
    actual = volume_distributions.brute_force_a(np.arange(100), b=b, target=target)
    # Assert
    assert actual == expected
