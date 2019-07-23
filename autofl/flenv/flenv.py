from typing import Any, Tuple, Union

import gym
import numpy as np
from absl import logging
from gym.envs.registration import register

from autofl.net import fc_compiled

from ..datasets.api import fashion_mnist_10s_600_load_splits
from ..fedml import Coordinator, Participant, RandomController

NUM_ROUNDS = 17


def register_gym_env():
    register(id="FederatedLearning-v0", entry_point="autofl.flenv:FederatedLearningEnv")


class FederatedLearningEnv(gym.Env):

    metadata = {"render.modes": ["human"]}

    def __init__(self) -> None:
        self.coordinator = init_fl()
        # Gym Env.observation_space and Env.action_space
        nvec = [2] * self.coordinator.num_participants()
        self.action_space = gym.spaces.MultiDiscrete(nvec)
        self.observation_space = gym.spaces.Box(
            low=0,
            high=1,
            shape=(NUM_ROUNDS, self.coordinator.num_participants()),
            dtype=np.float32,
        )
        self.num_rounds = NUM_ROUNDS
        self.state: np.ndarray = self._initial_state()
        self.round = 0
        self.prev_reward = 0.0

    def step(self, action: Union[np.ndarray, int]) -> Tuple[Any, float, bool, Any]:
        if isinstance(action, int):
            assert action >= 0
            assert action < self.num_participants()
            indices = [action]
        else:
            print(action.shape)
            assert action.shape == (self.num_participants(),)
            assert action.sum() >= 1  # There's at least one 1
            indices = action_to_indices(action).tolist()

        # Ask coordinator to train using the provided participants
        logging.info(
            "FlEnv: Train action {}, i.e. participants {}".format(action, indices)
        )
        self.coordinator.fit_round(indices)

        # Estimate loss and accuracy
        logging.info("FlEnv: Evaluate")
        # TODO consider: full validation (or test?) set evaluation after last round
        _, accuracy = self.coordinator.evaluate()
        # Reward: Gain in validation set accuracy (estimated)
        reward = accuracy - self.prev_reward
        self.prev_reward = reward

        # Update state: Override row of zeros with actual indices (i.e. the action taken)
        # This results in a soft Markovian state which is basically an action log
        self.state[self.round] = action

        # Done: Terminate when limit is reached
        self.round += 1
        done = self.round == self.num_rounds
        return np.copy(self.state), reward, done, None

    def reset(self) -> Any:
        self.state = self._initial_state()
        self.round = 0
        return np.copy(self.state)

    def render(self, mode="human") -> None:
        raise NotImplementedError()

    def num_participants(self) -> int:
        return self.coordinator.num_participants()

    def _initial_state(self) -> np.ndarray:
        return np.zeros((self.num_rounds, self.num_participants()))


def action_to_indices(action: np.ndarray) -> np.ndarray:
    return np.argwhere(action == 1).squeeze(axis=1)


def init_fl() -> Coordinator:
    # xy_splits, xy_test = data.generate_splits_mnist(num_splits=10)
    xy_splits, xy_test = fashion_mnist_10s_600_load_splits()
    # Init participants
    ps = []
    for xy_split in xy_splits:
        model = fc_compiled()
        p = Participant(model, xy_split[0], xy_split[1])
        ps.append(p)
    # Init coordinator
    # FIXME refactor: No controller needed
    controller = RandomController(10, 3)
    model = fc_compiled()
    return Coordinator(controller, model, ps, xy_test)
