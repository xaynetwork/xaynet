from typing import Tuple

import gym
import numpy as np
import tensorflow as tf
from absl import flags
from numpy import ndarray

from .. import flenv
from ..data import cifar10_random_splits_10
from ..fedml.fedml import Coordinator, Participant
from ..flenv.arch import Architecture, build_architecture, parse_arch_str

FLAGS = flags.FLAGS
PARTICIPANTS = 5


def main(_):
    # gym_autofl()
    autofl()


def gym_autofl():
    flenv.register_gym_env()
    e = gym.make("FederatedLearning-v0")
    print(e)


def build_model_and_print_summary():
    # Architecture
    print("#" * 80)
    if FLAGS.arch:
        print("Using user-provided arch:", FLAGS.arch)
        arch = parse_arch_str(FLAGS.arch)
    elif FLAGS.sample_random_arch:
        print("Using randomly sampled arch")
        arch = sample_architecture(num_layers=3)
    else:
        arch_strs = "0 1 0 2 0 0 3 0 0 0 4 0 0 0 0".split()
        print("Using hardcoded arch:", arch_strs)
        arch = parse_arch_str(arch_strs)
    print("Architecture:")
    print("\t architecture:", arch)
    print("\t num_layers:  ", arch.get_num_layers())
    # Model
    model = build_architecture(arch)
    optimizer = tf.keras.optimizers.SGD(lr=0.01, momentum=0.9)
    model.compile(
        optimizer=optimizer, loss="categorical_crossentropy", metrics=["accuracy"]
    )
    model.summary()


def autofl():
    print("\n\nStarting AutoFL\n")
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()

    print("Number of splits x/y train:", len(xy_splits))

    # Initialize participants and coordinator
    # Note that no initial model is provided to the constructors, the models
    # will be created and set by the agent.
    participants = []
    for x_split, y_split in xy_splits:
        participant = Participant(None, x_split, y_split)
        participants.append(participant)
    coordinator = Coordinator(None, participants)
    # AutoFL
    agent: Agent = RandomAgent(coordinator=coordinator)
    agent.train()
    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = agent.evaluate(x_test, y_test)
    print("\nFinal loss and accuracy:", loss, accuracy)


class Agent:
    def __init__(self, *args, **kwargs):
        pass

    def train(self, episodes: int):
        raise NotImplementedError("abstract method")

    def sample_architecture(self, num_layers: int) -> Architecture:
        raise NotImplementedError("abstract method")

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        raise NotImplementedError("abstract method")


class LstmAgent(Agent):
    def __init__(self, hidden_units=64):
        super().__init__(self)
        self.hidden_units = hidden_units

    def train(self, episodes: int):
        pass

    def sample_architecture(self, num_layers: int) -> Architecture:
        pass

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        pass


class RandomAgent(Agent):
    def __init__(self, coordinator: Coordinator):
        super().__init__(self)
        self.coordinator = coordinator

    def train(self, episodes=5):
        for episode in range(episodes):
            print("#" * 80)
            print("\tAutoFL Episode", episode)
            self._train_arch()

    def _train_arch(self) -> None:
        arch = self.sample_architecture(num_layers=2)

        def model_fn():
            model = build_architecture(arch)
            model.compile(
                optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
            )
            return model

        self.coordinator.replace_model(model_fn)
        self.coordinator.train_fl(num_rounds=2, C=3)

    def sample_architecture(self, num_layers: int) -> Architecture:
        return sample_architecture(num_layers=3)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        return self.coordinator.evaluate(x_test, y_test)


def sample_architecture(num_layers: int) -> Architecture:
    arch = Architecture()
    for layer_index in range(num_layers):
        op = np.random.randint(low=0, high=6, size=1)
        scs = np.random.randint(low=0, high=2, size=layer_index)  # Skip connections
        layer = np.hstack((op, scs))
        arch.add_layer(layer.tolist())
    return arch
