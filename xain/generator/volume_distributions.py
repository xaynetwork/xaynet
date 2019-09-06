from typing import Dict, List, Tuple

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

    def ed(x, a, b):
        return a * (b ** x)

    return [int(ed(x, a=a, b=b)) for x in xs]
