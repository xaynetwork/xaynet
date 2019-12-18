"""Provides participant API"""

from abc import ABC, abstractmethod
from typing import Dict, List, Tuple

from numpy import ndarray


class Participant(ABC):
    """An abstract participant for federated learning."""

    @abstractmethod
    def train_round(
        self, weights: List[ndarray], epochs: int, epoch_base: int
    ) -> Tuple[List[ndarray], int, Dict[str, List[ndarray]]]:
        # pylint: disable=line-too-long
        """Train the model in a federated learning round.

        A global model is given in terms of its `weights` and it is trained on local data for a
        number of `epochs`. The weights of the updated local model are returned together with the
        number of samples in the train dataset and a set of metrics.

        Args:
            weights (~typing.List[~numpy.ndarray]): The weights of the global model.
            epochs (int): The number of epochs to be trained.
            epoch_base (int): The epoch base number for the optimizer state (in case of epoch
                dependent optimizer parameters).

        Returns:
            ~typing.Tuple[~typing.List[~numpy.ndarray], int, ~typing.Dict[str, ~typing.List[~numpy.ndarray]]]:
                The updated model weights, the number of training samples and the gathered metrics.
        """
        # pylint: enable=line-too-long
