import random
from abc import ABC
from typing import List

import numpy as np

from .ops import federated_averaging


class Controller(ABC):
    def __init__(self, num_participants) -> None:
        self.num_participants = num_participants

    def indices(self) -> List[int]:
        raise NotImplementedError("not implemented")

    @staticmethod
    def aggregate(thetas):
        theta_prime = federated_averaging(thetas)
        return theta_prime


class RandomController(Controller):
    def __init__(self, num_participants: int, C: int):
        super().__init__(num_participants)
        self.C = C

    def indices(self) -> List[int]:
        return random.sample(range(0, self.num_participants), self.C)


class RoundRobinController(Controller):
    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.next_index: int = 0

    def indices(self) -> List[int]:
        next_index = self.next_index
        self.next_index = (next_index + 1) % self.num_participants
        return [next_index]


class CycleRandomController(Controller):
    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.cycle: np.ndarray = np.array([])

    def indices(self) -> List[int]:
        if self.cycle.size == 0:
            self.cycle = np.random.permutation(self.num_participants)
        next_index = self.cycle[0]
        self.cycle = self.cycle[1:]
        return [next_index]
