from typing import Tuple

import torch

from autofl.agent.transition_buffer import TransitionBuffer
from autofl.types import Transition


class TorchTransitionBuffer:
    def __init__(self, capacity: int) -> None:
        self.tb: TransitionBuffer = TransitionBuffer(capacity=capacity)

    def store(self, transition: Transition) -> None:
        self.tb.store(transition)

    def sample(
        self, k: int, device=torch.device("cpu")
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        transitions = self.tb.sample(k)

        states = torch.from_numpy(transitions[0]).float().to(device)
        actions = torch.from_numpy(transitions[1]).long().to(device)
        rewards = torch.from_numpy(transitions[2]).float().to(device)
        next_states = torch.from_numpy(transitions[3]).float().to(device)
        dones = torch.from_numpy(transitions[4]).float().to(device)

        return states, actions, rewards, next_states, dones

    def __len__(self) -> int:
        return len(self.tb)
