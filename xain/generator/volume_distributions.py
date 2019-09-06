from typing import Dict, List, Tuple

import matplotlib.pyplot as plt
import numpy as np

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


def fashion_mnist_100p():
    return _generate_100p_volume_distributions(bs_fashion_mnist)


def cifar_10_100p():
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


def plot_fashion_mnist_dist():
    dists = fashion_mnist_100p()
    xs = np.arange(100)
    plt.figure()
    legend = []
    for b, dist in dists:
        legend.append(str(b))
        plt.plot(xs, np.array(dist))
    plt.legend(legend, loc="upper left")
    plt.show()


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


if __name__ == "__main__":
    print("Fashion-MNIST:")
    brute_force_a_for_fashion_mnist()
    print("CIFAR-10:")
    brute_force_a_for_cifar_10()
    print("Plot Fashion-MNIST volume distributions")
    plot_fashion_mnist_dist()
