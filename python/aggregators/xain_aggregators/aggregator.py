from abc import ABC, abstractmethod
from typing import Optional


class AggregatorABC(ABC):
    @abstractmethod
    def aggregate(self) -> bytes:
        raise NotImplementedError()

    @abstractmethod
    def add_weights(self, data: bytes) -> bool:
        raise NotImplementedError()

    @abstractmethod
    def reset(self, global_weights: Optional[bytes]) -> None:
        raise NotImplementedError()

    @abstractmethod
    def get_global_weights(self) -> bytes:
        raise NotImplementedError()
