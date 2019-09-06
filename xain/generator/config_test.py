from xain.generator import volume_distributions

from .config import dist_to_indicies


def test_dist_to_indicies():
    # Prepare
    dists = (
        volume_distributions.cifar_10_100p() + volume_distributions.fashion_mnist_100p()
    )

    assert len(dists) == 20

    for _, dist in dists:
        indices = dist_to_indicies(dist)
        assert len(indices) == len(dist) - 1
