from typing import List

from autofl.types import KerasWeights

from . import ops


def weighted_avg(thetas: List[KerasWeights]) -> KerasWeights:
    # FIXME implement weighting
    return ops.federated_averaging(thetas)
