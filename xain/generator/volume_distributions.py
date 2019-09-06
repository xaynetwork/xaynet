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


def fashion_mnist_100p():
    return _generate_100p_volume_distributions(bs_fashion_mnist)


def _generate_100p_volume_distributions(
    bs: Dict[float, float]
) -> List[Tuple[float, List[int]]]:
    dists = []
    for b in bs:
        a = bs[b]
        dist = _generate_volume_distribution(np.arange(100), a, b)
        dists.append((b, dist))
    return dists


def _generate_volume_distribution(xs: np.ndarray, a: float, b: float) -> List[int]:
    assert xs.ndim == 1

    def ed(x, a, b):
        return a * (b ** x)

    return [int(ed(x, a=a, b=b)) for x in xs]
