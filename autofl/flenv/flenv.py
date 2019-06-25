import random
from typing import Any, Tuple

import gym
from gym.envs.registration import register


def register_gym_env():
    register(id="FederatedLearning-v0", entry_point="autofl.flenv:FederatedLearningEnv")


class FederatedLearningEnv(gym.Env):
    metadata = {"render.modes": ["human"]}

    def __init__(self) -> None:
        # TODO setup participants and coordinator
        # TODO load data for each participant
        print("FederatedLearningEvn initialized")

    def step(self, action: str) -> Tuple[Any, float, bool, Any]:
        return None, random.random(), False, None

    def reset(self) -> Any:
        return None

    def render(self, mode="human") -> None:
        raise NotImplementedError()
