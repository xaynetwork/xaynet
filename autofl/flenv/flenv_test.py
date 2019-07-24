import gym
import numpy as np

from autofl import flenv

from .flenv import action_to_indices


@pytest.mark.integration
def test_FederatedLearningEnv_init_reset():
    # Prepare
    flenv.register_gym_env()
    # Execute
    env = gym.make("FederatedLearning-v0")
    actual = env.reset()
    # Assert
    assert isinstance(env, flenv.FederatedLearningEnv)
    assert actual.shape[1] == env.num_participants()


def test_action_to_indices_1():
    # Prepare
    action = np.array([0, 1, 0, 0, 0, 0])
    expected = np.array([1])
    # Execute
    actual = action_to_indices(action)
    # Assert
    np.testing.assert_equal(actual, expected)


def test_action_to_indices_n():
    # Prepare
    action = np.array([0, 1, 0, 0, 1, 0])
    expected = np.array([1, 4])
    # Execute
    actual = action_to_indices(action)
    # Assert
    np.testing.assert_equal(actual, expected)
