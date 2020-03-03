from typing import TypeVar
from abc import ABC, abstractmethod


class TrainingResultABC(ABC):
    @abstractmethod
    def tobytes(self) -> bytes:
        raise NotImplementedError


# pylint: disable=invalid-name
T = TypeVar("T", bound="TrainingInput")


class TrainingInputABC(ABC):
    @staticmethod
    @abstractmethod
    def frombytes(data: bytes) -> T:
        raise NotImplementedError

    @abstractmethod
    def is_initialization_round(self) -> bool:
        return False
