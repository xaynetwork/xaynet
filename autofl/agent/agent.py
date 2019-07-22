from abc import ABC

import gym
import numpy as np
from absl import logging

from autofl.types import Transition

from ..flenv import register_gym_env
from .transition_buffer import TransitionBuffer


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


def main(_):
    logging.set_verbosity(logging.DEBUG)

    register_gym_env()
    env = gym.make("FederatedLearning-v0")
    logging.info("action_space:      {}".format(env.action_space))
    logging.info("observation_space: {}".format(env.observation_space))

    state = env.reset()
    logging.info(state)

    buffer = TransitionBuffer(capacity=3)

    for _ in range(3):
        action = np.array([0, 1, 0, 0, 0, 1, 0, 0, 0, 0])
        next_state, reward, done, _ = env.step(action)
        logging.info("s:\n{}\nr: {}\nd: {}".format(next_state, reward, done))

        transition: Transition = (state, action, reward, next_state, done)
        buffer.store(transition)
