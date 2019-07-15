import gym
from absl import logging

from ..flenv import register_gym_env


def main(_):
    logging.set_verbosity(logging.DEBUG)

    register_gym_env()
    env = gym.make("FederatedLearning-v0")
    print("action_space:      {}".format(env.action_space))
    print("observation_space: {}".format(env.observation_space))
