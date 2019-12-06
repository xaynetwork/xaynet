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

    def __init__(
        self, participants_ids: List[str], fraction_of_participants: float = 1.0
    ) -> None:
        self.participants_ids = participants_ids
        self.fraction_of_participants = fraction_of_participants
        self.num_ids_to_select = self.get_num_ids_to_select()

    def get_num_ids_to_select(self) -> int:
        """Calculates how many participant ids need to be selected.

        Returns:
            :obj:`int`: Number of participant ids to be selected
        """
        raw_num_ids_to_select = (
            len(self.participants_ids) * self.fraction_of_participants
        )
        max_valid_value = max(1, np.ceil(raw_num_ids_to_select))
        minimum_valid_value = min(len(self.participants_ids), max_valid_value)
        return int(minimum_valid_value)

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
        return np.random.choice(
            self.participants_ids, size=self.num_ids_to_select, replace=False
        )
