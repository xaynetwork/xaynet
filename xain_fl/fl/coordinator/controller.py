"""Provides an abstract base class Controller and the RandomController
currently used by the Coordinator."""

from abc import ABC, abstractmethod
from typing import List

import numpy as np


# TODO: raise exceptions for invalid attribute values: https://xainag.atlassian.net/browse/XP-387
class Controller(ABC):
    """Abstract base class which provides an interface to the coordinator that
    enables different selection strategies.

    Attributes:
        fraction_of_participants (:obj:`float`, optional): The fraction of total
            participant IDs to be selected. Defaults to 1.0, meaning that
            all participant IDs will be selected. It must be in the (0.0, 1.0] interval.
    """

    def __init__(self, fraction_of_participants: float = 1.0) -> None:
        """[summary]

        .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

        Args:
            fraction_of_participants (float): [description]. Defaults to 1.0.
        """

        self.fraction_of_participants: float = fraction_of_participants

    def get_num_ids_to_select(self, len_participant_ids: int) -> int:
        """Calculates how many participant IDs need to be selected.

        Args:
            len_participant_ids (:obj:`int`): The length of the list of IDs of all the
                available participants.

        Returns:
            :obj:`int`: Number of participant IDs to be selected
        """
        raw_num_ids_to_select = len_participant_ids * self.fraction_of_participants
        max_valid_value = max(1, np.ceil(raw_num_ids_to_select))
        minimum_valid_value = min(len_participant_ids, max_valid_value)
        return int(minimum_valid_value)

    @abstractmethod
    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Returns the selected indices of next round.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of all the
                available participants, a subset of which will be selected.

        Returns:
            :obj:`list` of :obj:`str`: List of selected participant IDs
        """
        raise NotImplementedError("not implemented")


class IdController(Controller):
    """[summary

    ... todo: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
    """

    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Selects all given participants.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of all the
                available participants, a subset of which will be selected.

        Returns:
            :obj:`list` of :obj:`str`: List of selected participant IDs
        """

        return participant_ids


class RandomController(Controller):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        Controller ([type]): [description]
    """

    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Randomly samples self.num_ids_to_select from the population of participants_ids,
        without replacement.

        Args:
            participant_ids (:obj:`list` of :obj:`str`): The list of IDs of all the
                available participants, a subset of which will be selected.

        Returns:
            :obj:`list` of :obj:`str`: List of selected participant IDs
        """

        num_ids_to_select = self.get_num_ids_to_select(len(participant_ids))
        return np.random.choice(participant_ids, size=num_ids_to_select, replace=False)
