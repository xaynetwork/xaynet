import random
from collections import deque
from typing import Deque, Tuple

import numpy as np

from autofl.types import Transition


class TransitionBuffer:
    def __init__(self, capacity: int) -> None:
        self.buffer: Deque = deque(maxlen=capacity)

    def store(self, transition: Transition) -> None:
        assert len(transition) == 5
        self.buffer.append(transition)

    def sample(
        self, k: int
    ) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
        transitions = random.sample(self.buffer, k=k)

        states = np.vstack([t[0] for t in transitions])
        actions = np.vstack([t[1] for t in transitions])
        rewards = np.vstack([t[2] for t in transitions])
        next_states = np.vstack([t[3] for t in transitions])
        dones = np.vstack([t[4] for t in transitions]).astype(np.uint8)

        return states, actions, rewards, next_states, dones

    def __len__(self) -> int:
        return len(self.buffer)
