from abc import ABC, abstractmethod


class TrainingResultABC(ABC):
    @abstractmethod
    def tobytes(self) -> bytes:
        raise NotImplementedError


class TrainingInputABC(ABC):
    @abstractmethod
    def is_initialization_round(self) -> bool:
        return False
