# Derived from: https://github.com/udacity/deep-reinforcement-learning/blob/master/dqn/solution/dqn_agent.py  # pylint: disable-msg=line-too-long

import random
from typing import cast

import numpy as np
import torch
import torch.nn.functional as F
import torch.optim as optim

from autofl.types import Transition

from .agent import Agent
from .dqn_torch import DQN, SeqDQN
from .transition_buffer_torch import TorchTransitionBuffer

CAPACITY = int(1e5)
BATCH_SIZE = 64
GAMMA = 0.99
TAU = 1e-3
LEARNING_RATE = 5e-4
UPDATE_EVERY = 4
DOUBLE_Q_LEARNING = True
DEFAULT_FNAME = "ckpt_torch.pth"

DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")


# pylint: disable-msg=too-many-arguments,too-many-instance-attributes
class TorchAgent(Agent):
    def __init__(
        self,
        input_shape,
        num_actions: int,
        batch_size: int = BATCH_SIZE,
        seed: int = 0,
        seqdqn: bool = False,
    ) -> None:
        self.seqdqn = seqdqn
        random.seed(seed)
        self.num_actions = num_actions
        # DQN: Policy & target network
        if seqdqn:
            self.dqn_policy = SeqDQN(input_shape, num_actions, seed=0).to(DEVICE)
            self.dqn_target = SeqDQN(input_shape, num_actions, seed=0).to(DEVICE)
        else:
            self.dqn_policy = DQN(input_shape[0], num_actions, seed=0).to(DEVICE)
            self.dqn_target = DQN(input_shape[0], num_actions, seed=0).to(DEVICE)
        self.optimizer = optim.Adam(self.dqn_policy.parameters(), lr=LEARNING_RATE)
        # Transition buffer
        self.buffer = TorchTransitionBuffer(CAPACITY)
        self.t_step = 0  # Initialize time step (for updating every UPDATE_EVERY steps)
        self.batch_size = batch_size

    def save_policy(self, fname: str = DEFAULT_FNAME) -> None:
        torch.save(self.dqn_policy.state_dict(), fname)

    def load_policy(self, fname: str = DEFAULT_FNAME) -> None:
        self.dqn_policy.load_state_dict(torch.load(fname))

    def action_discrete(self, observation, epsilon) -> int:
        # Epsilon-greedy action selection
        if random.random() <= epsilon:
            random_action = random.choice(np.arange(self.num_actions))
            return int(random_action)
        # Compute Q(s_t)
        state = torch.from_numpy(observation).float().unsqueeze(0).to(DEVICE)
        if self.seqdqn:
            state = state.unsqueeze(0)
        self.dqn_policy.eval()
        with torch.no_grad():
            action_values = self.dqn_policy(state)
        self.dqn_policy.train()
        # Greedy action
        greedy_action = np.argmax(action_values.cpu().data.numpy())
        return int(greedy_action)

    def action_multi_discrete(self, observation, epsilon) -> np.ndarray:
        # Epsilon-greedy action selection
        if random.random() <= epsilon:
            random_indices = np.random.choice(np.arange(10), 3)
            return multi_hot(random_indices, 10)
        # Compute Q(s_t)
        state = torch.from_numpy(observation).float().unsqueeze(0).to(DEVICE)
        if self.seqdqn:
            state = state.unsqueeze(0)
        self.dqn_policy.eval()
        with torch.no_grad():
            action_values = self.dqn_policy(state)
        self.dqn_policy.train()
        # Greedy action: N largest Q(s_t, a)
        q = action_values.squeeze().cpu().data.numpy()
        largest = n_largest(q, 3)
        return multi_hot(largest, 10)

    def update(self, transition: Transition) -> None:
        # Save experience in transition buffer
        self.buffer.store(transition)
        # Improve policy every UPDATE_EVERY time steps
        self.t_step = (self.t_step + 1) % UPDATE_EVERY
        if self.t_step == 0:
            # Refine DQN if enough examples are available in buffer
            if len(self.buffer) > self.batch_size:
                self._train(GAMMA)

    # pylint: disable-msg=too-many-locals
    def _train(self, gamma: float = GAMMA) -> None:
        # FIXME implement multi-discrete action support
        # Sample from transition buffer
        experiences = self.buffer.sample(k=self.batch_size)
        states, actions, rewards, next_states, dones = experiences
        dones_bytes = cast(torch.Tensor, dones.type(dtype=torch.uint8))

        # Compute Q(s_t, a): Model computes Q(s_t), then we select the columns of actions taken
        state_action_values = self.dqn_policy(states).gather(1, actions)

        # Compute V(s_{t+1}) for all next states
        if DOUBLE_Q_LEARNING:
            # Pick action using Policy Network
            next_state_predictions = self.dqn_policy(next_states)
            next_state_actions = next_state_predictions.argmax(1)

            # Evaluate action value using Target Network
            next_state_action_values = self.dqn_target(next_states)
            next_state_values_unsqueezed = next_state_action_values.gather(
                1, next_state_actions.view(-1, 1)
            )
            next_state_values = next_state_values_unsqueezed.squeeze().detach()
        else:
            next_state_predictions = self.dqn_target(next_states)
            next_state_max_prediction = next_state_predictions.max(1)
            next_state_values = next_state_max_prediction[0].detach()
        next_state_values[dones_bytes.squeeze()] = 0

        # Compute the expected Q values ("target" for calculating the loss)
        expected_state_action_values = rewards.squeeze() + (next_state_values * gamma)

        # Compute MSE
        # TODO Use Huber loss
        loss = F.mse_loss(
            state_action_values, expected_state_action_values.unsqueeze(1)
        )

        # Optimize policy network
        self.optimizer.zero_grad()
        loss.backward()
        # # Gradient clipping
        # for param in self.dqn_policy.parameters():
        #    param.grad.data.clamp_(-1, 1)
        self.optimizer.step()

        # Update target network
        soft_update(self.dqn_policy, self.dqn_target, TAU)


def soft_update(
    dqn_policy: torch.nn.Module, dqn_target: torch.nn.Module, tau: float
) -> None:
    """
    Update weights by copying from policy model to target model according to
    interpolation parameter tau:
        θ_target = τ*θ_policy + (1-τ)*θ_target
    """
    w_policy = dqn_policy.parameters()
    w_target = dqn_target.parameters()
    for target_param, policy_param in zip(w_target, w_policy):
        w_prime = tau * policy_param.data + (1.0 - tau) * target_param.data
        target_param.data.copy_(w_prime)


def n_largest(arr, n):
    return (-arr).argsort()[:n]


def multi_hot(indices: np.ndarray, size: int) -> np.ndarray:
    x = np.zeros((size)).astype(np.int64)
    x[indices] = 1
    return x
