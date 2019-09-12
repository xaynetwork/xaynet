from typing import Dict, List, Tuple

import matplotlib
import numpy as np
from absl import app, flags, logging

from xain.helpers import storage

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


def fashion_mnist_100p() -> List[Tuple[float, List[int]]]:
    return _generate_100p_volume_distributions(bs_fashion_mnist)


def cifar_10_100p() -> List[Tuple[float, List[int]]]:
    return _generate_100p_volume_distributions(bs_cifar_10)


def exponential_decay(x: int, a: float, b: float) -> float:
    return a * (b ** x)


def _generate_100p_volume_distributions(
    bs: Dict[float, float]
) -> List[Tuple[float, List[int]]]:
    dists = []
    for b, a in bs.items():
        dist = _generate_volume_distribution(np.arange(100), a, b)
        dists.append((b, dist))
    return dists


def _generate_volume_distribution(xs: np.ndarray, a: float, b: float) -> List[int]:
    assert xs.ndim == 1
    return [int(exponential_decay(x, a=a, b=b)) for x in xs]


def brute_force_a_for_fashion_mnist():
    for b in [1.0, 1.005, 1.01, 1.015, 1.02, 1.025, 1.03, 1.035, 1.04, 1.045]:
        a = brute_force_a(np.arange(100), b, target=54_000)
        print(f"{b}: {a},")


def brute_force_a_for_cifar_10():
    for b in [1.0, 1.005, 1.01, 1.015, 1.02, 1.025, 1.03, 1.035, 1.04, 1.045]:
        a = brute_force_a(np.arange(100), b, target=45_000)
        print(f"{b}: {a},")


# pylint: disable-msg=inconsistent-return-statements
def brute_force_a(xs, b: float, target: int, step=1.0, start=1):
    a_best = 1
    for a in np.arange(start, target, step):
        ys = [int(exponential_decay(x, a=a, b=b)) for x in xs]
        sum_ys = sum(ys)
        if sum_ys == target:
            return a
        if sum_ys < target:
            a_best = a
        else:
            # Recursive step
            return brute_force_a(xs, b, target, step / 10, start=a_best)


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


def plot_fashion_mnist_dist():
    dists = fashion_mnist_100p()
    xs = np.arange(100)
    plt.figure()
    legend = []
    for b, dist in dists:
        legend.append(str(b))
        plt.plot(xs, np.array(dist))
    plt.legend(legend, loc="upper left")

    fname_abspath = storage.get_abspath(
        "plot_fashion_mnist_partition_volume_dist", FLAGS.output_dir
    )
    plt.savefig(fname=fname_abspath, format=FORMAT)

    # FIXME: Matplotlib is currently using agg, which is a non-GUI
    #        backend, so cannot show the figure.
    # plt.show()

    return fname_abspath


def main():
    print("Fashion-MNIST:")
    # brute_force_a_for_fashion_mnist()
    print("CIFAR-10:")
    # brute_force_a_for_cifar_10()
    print("Plot Fashion-MNIST volume distributions")
    fmd_fpath = plot_fashion_mnist_dist()
    logging.info(f"Data plotted and saved in {fmd_fpath}")


if __name__ == "__main__":
    app.run(main=lambda _: main())
