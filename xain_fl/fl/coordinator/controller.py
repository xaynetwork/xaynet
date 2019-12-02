"""Provides an abstract base class Controller and multiple sub-classes
such as CycleRandomController.
"""
import random
from abc import ABC
from typing import List

import numpy as np


class Controller(ABC):
    """Abstract base class which provides an interface to the coordinator that
    enables different selection strategies.
    """

    def __init__(self, num_participants) -> None:
        self.num_participants = num_participants

    def indices(self, num_indices: int) -> List[int]:
        """Returns the selected indices of next round

        Args:
            num_indicies (int): Number of participants to select

        Returns:
            List[int]: Unordered list of selected indices
        """
        raise NotImplementedError("not implemented")


class RandomController(Controller):
    """Randomly selects indicies"""

    def indices(self, num_indices: int) -> List[int]:
        return random.sample(range(0, self.num_participants), num_indices)


class RoundRobinController(Controller):
    """Cycles through all indicies in an ordered fashion."""

    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.next_index: int = 0

    def indices(self, num_indices: int) -> List[int]:
        next_index = self.next_index
        self.next_index = (next_index + 1) % self.num_participants
        return [next_index]


class CycleRandomController(Controller):
    """Cycles through all indicies in a random fashion."""

    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.cycle: np.ndarray = np.array([])

    def indices(self, num_indices: int) -> List[int]:
        if self.cycle.size == 0:
            self.cycle = np.random.permutation(self.num_participants)
        next_index = self.cycle[0]
        self.cycle = self.cycle[1:]
        return [next_index]
