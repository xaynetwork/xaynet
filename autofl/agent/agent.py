from abc import ABC

import numpy as np

from autofl.types import Transition


class Agent(ABC):
    def action_discrete(self, observation, epsilon) -> int:
        raise NotImplementedError()

    def action_multi_discrete(self, observation, epsilon) -> np.ndarray:
        raise NotImplementedError()

    def update(self, transition: Transition) -> None:
        raise NotImplementedError()

    def save_policy(self, fname: str = "") -> None:
        raise NotImplementedError()

    def load_policy(self, fname: str = "") -> None:
        raise NotImplementedError()
