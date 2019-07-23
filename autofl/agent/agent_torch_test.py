import numpy as np

from .agent_torch import TorchAgent, multi_hot, n_largest


def test_TorchAgent_action_discrete_random():
    # Prepare
    input_size = (1, 10, 10)
    agent = TorchAgent(input_shape=input_size, num_actions=10, seqdqn=True)
    observation = np.random.randn(10, 10)

    # Execute
    a = agent.action_discrete(observation, epsilon=0.0)

    # Assert
    assert isinstance(a, int)
    assert a >= 0
    assert a < 10


def test_TorchAgent_action_discrete_greedy():
    # Prepare
    input_size = (1, 10, 10)
    agent = TorchAgent(input_shape=input_size, num_actions=10, seqdqn=True)
    observation = np.random.randn(10, 10)

    # Execute
    a = agent.action_discrete(observation, epsilon=1.0)

    # Assert
    assert isinstance(a, int)
    assert a >= 0
    assert a < 10


def test_TorchAgent_action_multi_discrete():
    # Prepare
    input_size = (1, 10, 10)
    agent = TorchAgent(input_shape=input_size, num_actions=10, seqdqn=True)
    observation = np.random.randn(10, 10)

    # Execute
    q = agent.action_multi_discrete(observation, epsilon=0.0)

    # Assert
    assert q.shape == (10,)
    assert np.sum(q) == 3


def test_n_largest():
    # Prepare
    x = np.array([0, 3, 2, 1, 5])

    # Execute
    n = n_largest(x, 3)

    # Assert
    assert sorted(n) == [1, 2, 4]


def test_multi_hot():
    # Prepare
    indices = np.array([4, 1, 2])
    expected = np.array([0, 1, 1, 0, 1]).astype(np.int64)

    # Execute
    actual = multi_hot(indices, 5)

    # Assert
    np.testing.assert_array_equal(actual, expected)
