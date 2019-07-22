import torch
import torch.nn as nn
import torch.nn.functional as F


class SeqDQN(nn.Module):
    # pylint: disable-msg=unused-argument
    def __init__(self, input_shape, num_actions: int = 10, seed=None) -> None:
        super(SeqDQN, self).__init__()
        if seed is not None:
            self.seed = torch.manual_seed(seed)
        self.conv1 = nn.Conv2d(1, 8, kernel_size=5)
        self.conv2 = nn.Conv2d(8, 16, kernel_size=3)
        self.fc1 = nn.Linear(256, 64)
        self.fc2 = nn.Linear(64, num_actions)

    def forward(self, *x):
        x = x[0]
        x = F.relu(self.conv1(x))
        x = F.relu(self.conv2(x))
        x = x.view(-1, 256)
        x = F.relu(self.fc1(x))
        x = self.fc2(x)
        return x


class DQN(nn.Module):
    def __init__(self, state_size: int, action_size: int, seed=None):
        super(DQN, self).__init__()
        if seed is not None:
            self.seed = torch.manual_seed(seed)
        intermediate_units = 64
        self.fc1 = nn.Linear(state_size, intermediate_units)
        self.fc2 = nn.Linear(intermediate_units, intermediate_units)
        self.fc_n = nn.Linear(intermediate_units, action_size)

    def forward(self, *x):
        x = x[0]
        x = F.relu(self.fc1(x))
        x = F.relu(self.fc2(x))
        x = self.fc_n(x)
        return x
