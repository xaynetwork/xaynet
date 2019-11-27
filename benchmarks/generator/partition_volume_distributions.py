from typing import Dict, List, Tuple

import matplotlib
import numpy as np
from absl import app, flags, logging

from benchmarks.helpers import storage

FLAGS = flags.FLAGS

matplotlib.use("AGG")

# To avoid issues with tkinter we need to set the renderer
# for matplotlib before importing pyplot
# As isort would move this line under the "import matplotlib"
# We need to skip isort explicitly
# pylint: disable-msg=wrong-import-position, wrong-import-order
import matplotlib.pyplot as plt  # isort:skip

FORMAT = "png"

bs_fashion_mnist: Dict[float, float] = {
    1.0: 540.0,
    1.005: 417.94000000000017,
    1.01: 317.03099999999995,
    1.015: 236.25199999999995,
    1.02: 173.123,
    1.025: 124.95999999999998,
    1.03: 89.00600000000003,
    1.035: 62.66,
    1.04: 43.67599999999998,
    1.045: 30.181500000000014,
}
"""Parameters b, a (key, value) for ~._exponential_decay for FashionMNIST
examples distribution. Can be calculated with ~._brute_force_a_for_fashion_mnist
"""


bs_cifar_10: Dict[float, float] = {
    1.0: 450.0,
    1.005: 348.351,
    1.01: 264.28,
    1.015: 196.89999999999995,
    1.02: 144.26099999999994,
    1.025: 104.15400000000004,
    1.03: 74.18436000000008,
    1.035: 52.22250000000001,
    1.04: 36.402,
    1.045: 25.154300000000013,
}
"""Parameters b, a (key, value) for ~._exponential_decay for CIFAR-10
examples distribution. Can be calculated with ~._brute_force_a_for_cifar_10
"""


def fashion_mnist_100p() -> List[Tuple[float, List[int]]]:
    """Returns the distribution of examples for a FashionMNIST dataset with 100 partitions

    Returns:
        List[Tuple[float, List[int]]]: List of tuples with (b, List[int]) where the list of
            ints describes the number of examples in the partition at the index of the int
    """
    return _generate_100p_volume_distributions(bs_fashion_mnist)


def cifar_10_100p() -> List[Tuple[float, List[int]]]:
    """Returns the distribution of examples for a CIFAR-10 dataset with 100 partitions

    Returns:
        List[Tuple[float, List[int]]]: List of tuples with (b, List[int]) where the list of
            ints describes the number of examples in the partition at the index of the int
    """
    return _generate_100p_volume_distributions(bs_cifar_10)


def _exponential_decay(x: int, a: float, b: float) -> float:
    return a * (b ** x)


def _generate_100p_volume_distributions(
    bs: Dict[float, float]
) -> List[Tuple[float, List[int]]]:
    """Returns the distribution of examples for a dataset with 100 partitions

    Args:
        bs (Dict[float, float]): Dict key will be passed as `b` and value as `a`
            to ~._exponential_decay function(x, a, b)

    Returns:
        List[Tuple[float, List[int]]]: List of tuples with (b, List[int]) where the list of
            ints describes the number of examples in the partition at the index of the int
    """
    dists = []
    for b, a in bs.items():
        dist = _generate_volume_distribution(np.arange(100), a, b)
        dists.append((b, dist))
    return dists


def _generate_volume_distribution(xs: np.ndarray, a: float, b: float) -> List[int]:
    assert xs.ndim == 1
    return [int(_exponential_decay(x, a=a, b=b)) for x in xs]


def _brute_force_a_for_fashion_mnist():
    for b in [1.0, 1.005, 1.01, 1.015, 1.02, 1.025, 1.03, 1.035, 1.04, 1.045]:
        a = _brute_force_a(np.arange(100), b, target=54_000)
        print(f"{b}: {a},")


def _brute_force_a_for_cifar_10():
    for b in [1.0, 1.005, 1.01, 1.015, 1.02, 1.025, 1.03, 1.035, 1.04, 1.045]:
        a = _brute_force_a(np.arange(100), b, target=45_000)
        print(f"{b}: {a},")


# pylint: disable-msg=inconsistent-return-statements
def _brute_force_a(xs, b: float, target: int, step=1.0, start=1):
    a_best = 1
    for a in np.arange(start, target, step):
        ys = [int(_exponential_decay(x, a=a, b=b)) for x in xs]
        sum_ys = sum(ys)
        if sum_ys == target:
            return a
        if sum_ys < target:
            a_best = a
        else:
            # Recursive step
            return _brute_force_a(xs, b, target, step / 10, start=a_best)


def b_to_str(b: float):
    b_str = f"{b:<f}"
    return b_str[:5].replace(".", "_")


def dist_to_indicies(dist: List[int]) -> List[int]:
    indices = [0] * len(dist)
    for i, _ in enumerate(dist):
        if i == 0:
            indices[i] = dist[i]
        else:
            indices[i] = indices[i - 1] + dist[i]

    assert indices[-1] == sum(dist)

    # Exclude last element as indices only mark start of section
    return indices[:-1]


def _plot_fashion_mnist_dist():
    dists = fashion_mnist_100p()
    xs = np.arange(100)
    plt.figure()
    legend = []
    for b, dist in dists:
        legend.append(str(b))
        plt.plot(xs, np.array(dist), "o", markersize=1.0)
    plt.legend(legend, loc="upper left")

    plt.xlabel("Partition ID")
    plt.ylabel("Examples")

    dname = storage.create_output_subdir("partition_volume_distributions")
    fname = storage.fname_with_default_dir("plot-part-vol.png", dname)

    plt.savefig(fname=fname, format=FORMAT)

    # FIXME: Matplotlib is currently using agg, which is a non-GUI
    #        backend, so cannot show the figure.
    # plt.show()

    return fname


def main():
    """Calculates and prints a, b parameters for calculation of
    volume distributions
    """
    print("Fashion-MNIST:")
    _brute_force_a_for_fashion_mnist()
    print("CIFAR-10:")
    _brute_force_a_for_cifar_10()
    print("Plot Fashion-MNIST volume distributions")
    fmd_fpath = _plot_fashion_mnist_dist()
    logging.info(f"Data plotted and saved in {fmd_fpath}")


if __name__ == "__main__":
    app.run(main=lambda _: main())
