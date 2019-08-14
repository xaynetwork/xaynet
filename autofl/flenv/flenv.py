from typing import Any, Tuple, Union

import gym
import numpy as np
from absl import logging
from gym.envs.registration import register

from autofl.datasets import load_splits
from autofl.fl.coordinator import Coordinator, RandomController
from autofl.fl.participant import Participant
from autofl.net import orig_cnn_compiled

NUM_ROUNDS = 10  # FIXME: 40?


def register_gym_env():
    register(id="FederatedLearning-v0", entry_point="autofl.flenv:FederatedLearningEnv")


# pylint: disable-msg=too-many-instance-attributes
class FederatedLearningEnv(gym.Env):

    metadata = {"render.modes": ["human"]}

    def __init__(self) -> None:
        coordinator, xy_val, xy_test = init_fl()
        self.coordinator = coordinator
        self.xy_val = xy_val
        self.xy_test = xy_test
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

        # Update state: Override row of zeros with actual indices (i.e. the action taken)
        # This results in a soft Markovian state which is basically an action log
        self.state[self.round] = action

        # Done: Terminate when limit is reached
        self.round += 1
        done = self.round == self.num_rounds

        # Estimate loss and accuracy
        # TODO consider: full validation (or test?) set evaluation after last round
        if done:
            logging.info("FlEnv: Evaluate on test set")
            _, accuracy = self.coordinator.evaluate(self.xy_test)
        else:
            logging.info("FlEnv: Evaluate on validation set")
            _, accuracy = self.coordinator.evaluate(self.xy_val)
        # Reward: Gain in validation set accuracy (estimated)
        # FIXME this mixes validation and test set accuracy
        reward = accuracy - self.prev_reward
        self.prev_reward = reward

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


def init_fl() -> Tuple[Coordinator, Any, Any]:
    xy_splits, xy_val, xy_test = load_splits("fashion_mnist_10s_600")
    assert xy_splits is not None, "xy_splits is None"
    assert xy_val is not None, "xy_val is None"
    assert xy_test is not None, "xy_test is None"
    # Init participants
    participants = []
    for cid, xy_train in enumerate(xy_splits):
        model = orig_cnn_compiled()
        p = Participant(str(cid), model, xy_train, xy_val)
        participants.append(p)
    # Init coordinator
    # FIXME refactor: No controller needed
    controller = RandomController(10)
    model = orig_cnn_compiled()
    return (
        Coordinator(controller, model, participants, C=0.3, E=1, xy_val=xy_val),
        xy_val,
        xy_test,
    )
