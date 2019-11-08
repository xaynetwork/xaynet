import math
from typing import Callable


def exp_decay_fn(epoch_base: int, lr_initial: float, k: float) -> Callable:
    def exp_decay(epoch_optimizer: int) -> float:
        epoch = epoch_base + epoch_optimizer
        return lr_initial * math.exp(-k * epoch)

    return exp_decay
