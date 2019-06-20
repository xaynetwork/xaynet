import random

import gym
from gym.envs.registration import register


def register_gym_env():
    register(id="FederatedLearning-v0", entry_point="autofl.flenv:FederatedLearningEnv")


class FederatedLearningEnv(gym.Env):
    metadata = {"render.modes": ["human"]}

    def __init__(self):
        # TODO setup participants and coordinator
        # TODO load data for each participant
        print("FederatedLearningEvn initialized")

    def step(self, action):
        return None, random.random(), False, None

    def reset(self):
        return None

    def render(self, mode="human"):
        raise NotImplementedError()
