import gym

from autofl import flenv


def test_FederatedLearningEnv_init():
    # Prepare
    flenv.register_gym_env()
    # Execute
    env = gym.make("FederatedLearning-v0")
    # Assert
    assert isinstance(env, flenv.FederatedLearningEnv)
