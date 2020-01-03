"""Provides an abstract base class Controller and multiple sub-classes
such as CycleRandomController.
"""
from abc import ABC, abstractmethod
from typing import List

import numpy as np


class Controller(ABC):
    """Abstract base class which provides an interface to the coordinator that
    enables different selection strategies.

    Attributes:
        fraction_of_participants (:obj:`float`, optional): The fraction of total
            participant ids to be selected. Defaults to 1.0, meaning that
            all participant ids will be selected.
    """

    def __init__(self, fraction_of_participants: float = 1.0) -> None:
        self.fraction_of_participants: float = fraction_of_participants

    def get_num_ids_to_select(self, participant_ids: List[str]) -> int:
        """Calculates how many participant ids need to be selected.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of the
                all the available participants, to calculate its length.

        Returns:
            :obj:`int`: Number of participant ids to be selected
        """
        raw_num_ids_to_select = len(participant_ids) * self.fraction_of_participants
        max_valid_value = max(1, np.ceil(raw_num_ids_to_select))
        minimum_valid_value = min(len(participant_ids), max_valid_value)
        return int(minimum_valid_value)

    @abstractmethod
    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Returns the selected indices of next round.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of the
                all the available participants, a subset of which will be selected.

        Returns:
            :obj:`list` of :obj:`str`: List of selected participant ID's
        """
        raise NotImplementedError("not implemented")


class RandomController(Controller):
    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Randomly samples self.num_ids_to_select from the population of participants_ids,
        without replacement.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of the
                all the available participants, a subset of which will be selected.

        Returns:
            :obj:`list` of :obj:`str`: List of selected participant ID's
        """
        num_ids_to_select = self.get_num_ids_to_select(participant_ids)
        return np.random.choice(participant_ids, size=num_ids_to_select, replace=False)
