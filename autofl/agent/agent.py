import gym
import numpy as np
from absl import logging

from ..flenv import register_gym_env


def main(_):
    logging.set_verbosity(logging.DEBUG)

    register_gym_env()
    env = gym.make("FederatedLearning-v0")
    logging.info("action_space:      {}".format(env.action_space))
    logging.info("observation_space: {}".format(env.observation_space))

    state = env.reset()
    logging.info(state)

    action = np.array([0, 1, 0, 0, 0, 1, 0, 0, 0, 0])
    state, reward, done, _ = env.step(action)
    logging.info("s:\n{}\nr: {}\nd: {}".format(state, reward, done))
