import numpy as np
import pytest

from . import partition_volume_distributions as pvd


def test_fashion_mnist_distributions():
    # Prepare
    expected_sum = 54_000

    # Execute
    dists = pvd.fashion_mnist_100p()

    # Assert
    assert len(dists) == 10
    for dist in dists:
        assert sum(dist[1]) == expected_sum


def test_cifar_10_distributions():
    # Prepare
    expected_sum = 45_000

    # Execute
    dists = pvd.cifar_10_100p()

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
    # pylint: disable-msg=protected-access
    actual = pvd._brute_force_a(np.arange(100), b=b, target=target)
    # Assert
    assert actual == expected


def test_dist_to_indicies():
    # Prepare
    dists = pvd.cifar_10_100p() + pvd.fashion_mnist_100p()

    assert len(dists) == 20

    for _, dist in dists:
        indices = pvd.dist_to_indicies(dist)
        assert len(indices) == len(dist) - 1
