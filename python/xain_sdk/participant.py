from abc import ABC, abstractmethod
from typing import Any, Dict, List, Tuple, TypeVar, cast
import uuid

import numpy as np
from numpy import ndarray


class Participant(ABC):
    """An abstract participant for federated learning.
    """

    def __init__(self) -> None:
        """Initialize a participant."""

        super(Participant, self).__init__()

    @abstractmethod
    def init_weights(self) -> ndarray:
        """Initialize the weights of a model.

        The model weights are freshly initialized according to the participant's model
        definition and are returned without training.

        Returns:
            The newly initialized model weights.
        """

    @abstractmethod
    def train_round(
        self, weights: ndarray, epochs: int, epoch_base: int
    ) -> Tuple[ndarray, int]:
        """Train a model in a federated learning round.

        A model is given in terms of its weights and the model is trained on the
        participant's dataset for a number of epochs. The weights of the updated model
        are returned in combination with the number of samples of the train dataset.

        Any metrics that should be returned to the coordinator must be gathered via the
        participant's update_metrics() utility method per epoch.

        Args:
            weights: The weights of the model to be trained.
            epochs: The number of epochs to be trained.
            epoch_base: The global training epoch number.

        Returns:
            The updated model weights and the number of training samples.
        """


import enum


class State(enum.Enum):
   Waiting = 1
   PreTraining = 2
   Training= 3
   PostTraining = 4
   Finished = 5


class InternalParticipant:

    def __init__(self, participant: Participant):
        self._state_lock = threading.Lock()
        self.state = State.Waiting
        self.participant = participant
