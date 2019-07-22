import numpy as np
import torch

from .dqn_torch import SeqDQN


def test_SeqDQN_forward_1():
    # Prepare
    DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    input_size = (1, 10, 10)
    dqn = SeqDQN(input_shape=input_size, num_actions=10, seed=0).to(DEVICE)
    x_np = np.random.randn(1, 1, 10, 10)
    state = torch.from_numpy(x_np).float().to(DEVICE)

    # Execute
    q = dqn(state)

    # Assert
    assert q.shape == torch.Size([1, 10])


def test_SeqDQN_forward_n():
    # Prepare
    N = 3
    DEVICE = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    input_size = (1, 10, 10)
    dqn = SeqDQN(input_shape=input_size, num_actions=10, seed=0).to(DEVICE)
    x_np = np.random.randn(N, 1, 10, 10)
    state = torch.from_numpy(x_np).float().to(DEVICE)

    # Execute
    q = dqn(state)

    # Assert
    assert q.shape == torch.Size([N, 10])
