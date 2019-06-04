from random import randint
from typing import Tuple
from typing import List
import numpy as np
from numpy import ndarray
import tensorflow as tf
from autofl.mnist_f import mnist_f
from autofl.fedml import net


PARTICIPANTS = 10


def main():
    round_robin()


def individual():
    # Load data
    x_splits, y_splits, x_test, y_test = mnist_f.load_splits(num_splits=PARTICIPANTS)
    print("Number of splits x/y train:", len(x_splits), len(y_splits))
    # Create model
    model = net.fc()
    model.summary()
    # Train independent models on each data partition
    ps = []
    for x_split, y_split in zip(x_splits, y_splits):
        model = net.fc()  # Create a new model for each participant
        participant = Participant(model, x_split, y_split)
        ps.append(participant)
    # Train each model
    for p in ps:
        p.train(epochs=2)
    # Evaluate the individual performance of each model
    for i, p in enumerate(ps):
        loss, accuracy = p.evaluate(x_test, y_test)
        print("Participant", i, ":", loss, accuracy)


def round_robin():
    # Load data (multiple splits for training and one split for validation)
    x_splits, y_splits, x_test, y_test = mnist_f.load_splits(num_splits=PARTICIPANTS)
    print("Number of splits x/y train:", len(x_splits), len(y_splits))
    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = []
    for x_split, y_split in zip(x_splits, y_splits):
        model = net.fc()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    model = net.fc()  # This will act as the initial model
    coordinator = Coordinator(model, participants)
    # Start training
    coordinator.train(10)
    # Evaluate final model
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    print("Final loss and accuracy:", loss, accuracy)


class Participant:
    def __init__(
        self, model: tf.keras.Model, x_split: ndarray, y_split: ndarray
    ) -> None:
        assert x_split.shape[0] == y_split.shape[0]
        self.model = model
        self.x_split = x_split
        self.y_split = y_split
        self.history = None

    def update_model_parameters(self, theta: List[List[ndarray]]) -> None:
        _set_model_params(self.model, theta)

    def retrieve_model_parameters(self) -> List[List[ndarray]]:
        return _get_model_params(self.model)

    def train(self, epochs: int):
        x_train = self.x_split / 255.0
        y_train = self.y_split
        self.history = self.model.fit(x_train, y_train, epochs=epochs)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy


class Coordinator:
    def __init__(self, model: tf.keras.Model, participants: List[Participant]) -> None:
        self.model = model
        self.participants = participants

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def train(self, num_rounds: int) -> None:
        for round in range(num_rounds):
            # Select random participant
            random_index = randint(0, len(self.participants) - 1)
            print("Training round", str(round + 1), "- participant", random_index)
            participant = self.participants[random_index]
            # Push current model parameters to this participant
            theta = _get_model_params(self.model)
            participant.update_model_parameters(theta)
            # Train for a number of steps
            participant.train(1)  # TODO don't train a full episode, just a few steps
            # Pull updated model parameters from participant
            theta = participant.retrieve_model_parameters()
            # Update own model parameters
            _set_model_params(self.model, theta)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy


def _get_model_params(model: tf.keras.Model) -> List[List[ndarray]]:
    theta = []
    for layer in model.layers:
        theta.append(layer.get_weights())
    return theta


def _set_model_params(model: tf.keras.Model, theta: List[List[ndarray]]):
    for layer, layer_weights in zip(model.layers, theta):
        layer.set_weights(layer_weights)
