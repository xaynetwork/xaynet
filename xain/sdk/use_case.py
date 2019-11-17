"""Provides abstract base class use_case which provides an interface to the
participant runner"""
from abc import ABC, abstractmethod
from typing import List

from numpy import ndarray


class UseCase(ABC):
    def __init__(self, model):
        self.model = model

    @abstractmethod
    def set_weights(self, weights: List[ndarray]) -> None:
        """Will be called by the runner to set weights of model. The implementation
        should persist the weights so that they are used in a subsequent call to
        the train method.

        Args:
            weights (List[ndarray]): Model parameters
        """
        raise NotImplementedError()

    @abstractmethod
    def get_weights(self) -> List[ndarray]:
        """"Will be called by the runner after a train invocation to retrieve the
        weights of the model. The implementation should return the model weights.

        Returns:
            List[ndarray]: Model parameters
        """
        raise NotImplementedError()

    @abstractmethod
    def train(self):
        """Will be called by the runner to start the training of the model. The
        implementation should run the training when called and before returning
        from the method persist the weights so that in a subsequent get_weights
        call the correct weights are returned
        """
        raise NotImplementedError()
