"""Abstract Controller and the RandomController currently used by the Coordinator."""

from abc import ABC, abstractmethod
from typing import List

import numpy as np


# TODO: raise exceptions for invalid attribute values: https://xainag.atlassian.net/browse/XP-387
class Controller(ABC):
    """An interface to the coordinator that enables different selection strategies.

    Attributes:
        fraction_of_participants: The fraction of total participant IDs to be selected.
            Defaults to 1.0, meaning that all participant IDs will be selected. It must
            be in the (0.0, 1.0] interval.
    """

    def __init__(self, fraction_of_participants: float = 1.0) -> None:
        """Initialize the controller.

        Args:
            fraction_of_participants: The fraction of total participant IDs to be
                selected. Defaults to 1.0.
        """

        self.fraction_of_participants: float = fraction_of_participants

    def get_num_ids_to_select(self, len_participant_ids: int) -> int:
        """Calculate how many participant IDs need to be selected.

        Args:
            len_participant_ids: The length of the list of IDs of all the available
                participants.

        Returns:
            Number of participant IDs to be selected.
        """

        raw_num_ids_to_select = len_participant_ids * self.fraction_of_participants
        max_valid_value = max(1, np.ceil(raw_num_ids_to_select))
        minimum_valid_value = min(len_participant_ids, max_valid_value)
        return int(minimum_valid_value)

    @abstractmethod
    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Return the selected indices of next round.

        Args:
            participant_ids: The list of IDs of all the available participants, a subset
                of which will be selected.

        Returns:
            List of selected participant IDs.
        """

        raise NotImplementedError("not implemented")


class IdController(Controller):
    """A controller that selects all available participants."""

    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Select all given participants.

        Args:
            participant_ids: The list of IDs of all the available participants, a subset
                of which will be selected.

        Returns:
            List of selected participant IDs.
        """

        return participant_ids


class OrderController(Controller):
    """A controller that selects and orders all available participants."""

    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Select participants according to order.

        Args:
            participant_ids: The list of IDs of all the available participants, a subset
                of which will be selected.

        Returns:
            List of selected participant IDs.
        """

        num_ids_to_select = self.get_num_ids_to_select(len(participant_ids))
        sorted_ids = sorted(participant_ids)
        return sorted_ids[:num_ids_to_select]


class RandomController(Controller):
    """A controller that randomly selects a subset of all available participants."""

    def select_ids(self, participant_ids: List[str]) -> List[str]:
        """Randomly sample participants without replacement.

        The number of participants is determined according to the fraction of
        participants wrt. the number of available participants.

        Args:
            participant_ids: The list of IDs of all the available participants, from
                which a subset will be selected.

        Returns:
            List of selected participant IDs.
        """

        num_ids_to_select = self.get_num_ids_to_select(len(participant_ids))
        ids = np.random.choice(participant_ids, size=num_ids_to_select, replace=False)
        list_ids: List[str] = ids.tolist()
        return list_ids
