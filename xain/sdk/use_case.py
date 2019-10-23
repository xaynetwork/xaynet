from abc import ABC, abstractmethod
from typing import List

from numpy import ndarray


class UseCase(ABC):
    def __init__(self, model):
        self.model = model

    @abstractmethod
    def set_weights(self, weights: List[ndarray]) -> None:
        raise NotImplementedError()

    @abstractmethod
    def get_weights(self) -> List[ndarray]:
        raise NotImplementedError()

    @abstractmethod
    def train(self):
        raise NotImplementedError()
