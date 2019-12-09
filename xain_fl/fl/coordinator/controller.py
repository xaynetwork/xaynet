"""Provides an abstract base class Controller and multiple sub-classes
such as CycleRandomController.
"""
from abc import ABC, abstractmethod
from typing import List

import numpy as np


class Controller(ABC):
    """Abstract base class which provides an interface to the coordinator that
    enables different selection strategies.

    Args:
        participant_ids (:obj:`list` of :obj:`str`): The list of IDs of the
            all the available participants, a subset of which will be selected.
        fraction_of_participants (:obj:`float`, optional): The fraction of total
            participant ids to be selected. Defaults to 1.0, meaning that
            all participant ids will be selected.
    """

    def __init__(self, participants_ids: List[str], fraction_of_participants: float = 1.0) -> None:
        self.participants_ids = participants_ids
        self.fraction_of_participants = fraction_of_participants

    # TODO: make this a property?
    def get_num_ids_to_select(self) -> int:
        """Calculates how many participant ids need to be selected.

        Returns:
            :obj:`int`: Number of participant ids to be selected
        """

        raw_num_ids_to_select = len(self.participants_ids) * self.fraction_of_participants
        ceiling = np.ceil(raw_num_ids_to_select)
        return int(ceiling)

    @abstractmethod
    def select_ids(self) -> List[str]:
        """Returns the selected indices of next round

        Returns:
            :obj:`list` of :obj:`str`: Unordered list of selected ids
        """
        raise NotImplementedError("not implemented")


class RandomController(Controller):
    """Generates a random sample of the provided ids of size num_ids_to_select"""

    def select_ids(self) -> List[str]:
        num_ids_to_select = self.get_num_ids_to_select()
        return np.random.choice(self.participants_ids, size=num_ids_to_select, replace=False)


# TODO: refactor, legacy code
class RoundRobinController(Controller):
    """Cycles through all indicies in an ordered fashion."""

    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.next_index: int = 0

    def select_ids(self) -> List[str]:
        next_index = self.next_index
        self.next_index = (next_index + 1) % self.num_participants
        return [next_index]


# TODO: refactor, legacy code
class CycleRandomController(Controller):
    """Cycles through all indicies in a random fashion."""

    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants)
        self.cycle: np.ndarray = np.array([])

    def select_ids(self) -> List[str]:
        if self.cycle.size == 0:
            self.cycle = np.random.permutation(self.num_participants)
        next_index = self.cycle[0]
        self.cycle = self.cycle[1:]
        return [next_index]
