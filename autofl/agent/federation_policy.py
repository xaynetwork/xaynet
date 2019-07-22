import gym
from typing_extensions import Final

from autofl import flenv
from autofl.agent.train import train

from .agent_dqn import TorchAgent

LEARNING_RATE: Final = 5e-4
MAX_E: Final = 2000
MAX_T: Final = 100
EPSILON_INITIAL = 1.0
EPSILON_FINAL = 0.01

WATCH_UNTRAINED_AGENT = True
WATCH_TRAINED_AGENT = True


def main(_):
    # Environment
    flenv.register_gym_env()
    env = gym.make("FederatedLearning-v0")

    # Agent
    input_size = (1, 10, 10)
    agent = TorchAgent(input_shape=input_size, num_actions=10, seqdqn=True)

    # Training
    train(
        agent,
        env,
        max_e=MAX_E,
        max_t=MAX_T,
        epsilon_initial=EPSILON_INITIAL,
        epsilon_final=EPSILON_FINAL,
    )
