import random
from random import randint
from typing import Callable, List, Tuple

import tensorflow as tf
from numpy import ndarray

from autofl.data import data, prep
from autofl.fedml import net

PARTICIPANTS = 10


def main():
    federated_learning()


def individual():
    # Load data
    x_splits, y_splits, x_test, y_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
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
    x_splits, y_splits, x_test, y_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
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


def federated_learning():
    print("\n\nStarting federated learning\n")
    # Load data (multiple splits for training and one split for validation)
    x_splits, y_splits, x_test, y_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
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
    coordinator.train_fl(10, C=3)
    # Evaluate final model
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    print("\nFinal loss and accuracy:", loss, accuracy)


class Participant:
    def __init__(
        self, model: tf.keras.Model, x_split: ndarray, y_split: ndarray
    ) -> None:
        assert x_split.shape[0] == y_split.shape[0]
        self.model = model
        self.dataset = prep.init_dataset(x_split, y_split)
        self.history = None

    def replace_model(self, model: tf.keras.Model) -> None:
        self.model = model

    def update_model_parameters(self, theta: List[List[ndarray]]) -> None:
        _set_model_params(self.model, theta)

    def retrieve_model_parameters(self) -> List[List[ndarray]]:
        return _get_model_params(self.model)

    def train(self, epochs: int):
        self.history = self.model.fit(self.dataset, epochs=epochs, steps_per_epoch=8)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        # FIXME use Dataset
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy


class Coordinator:
    def __init__(self, model: tf.keras.Model, participants: List[Participant]) -> None:
        self.model = model
        self.participants = participants

    def replace_model(self, model_fn: Callable[..., tf.keras.Model]) -> None:
        self.model = model_fn()
        for p in self.participants:
            model = model_fn()
            p.replace_model(model)

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def train(self, num_rounds: int) -> None:
        for training_round in range(num_rounds):
            # Select random participant
            random_index = randint(0, len(self.participants) - 1)
            print(
                "\nTraining round",
                str(training_round + 1),
                "- participant",
                random_index,
            )
            participant = self.participants[random_index]
            # Push current model parameters to this participant
            theta = _get_model_params(self.model)
            participant.update_model_parameters(theta)
            # Train for a number of steps
            participant.train(1)  # TODO don't train a full episode, just a few steps
            # Pull updated model parameters from participant
            theta_prime = participant.retrieve_model_parameters()
            # Update own model parameters
            _set_model_params(self.model, theta_prime)

    def train_fl(self, num_rounds: int, C: int) -> None:
        for training_round in range(num_rounds):
            random_indices = random.sample(range(0, len(self.participants)), C)
            print(
                "\nTraining round",
                str(training_round + 1),
                "- participants",
                random_indices,
            )
            # Collect training results from the participants of this round
            thetas = []
            for index in random_indices:
                theta = self._single_step(index)
                thetas.append(theta)
            # Aggregate training results
            theta_prime = federated_averaging(thetas)
            # Update own model parameters
            _set_model_params(self.model, theta_prime)

    def _single_step(self, random_index: int) -> List[List[ndarray]]:
        participant = self.participants[random_index]
        # Push current model parameters to this participant
        theta = _get_model_params(self.model)
        participant.update_model_parameters(theta)
        # Train for a number of steps
        participant.train(1)  # TODO don't train a full episode, just a few steps
        # Pull updated model parameters from participant
        theta_prime = participant.retrieve_model_parameters()
        return theta_prime

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy


def federated_averaging(thetas: List[List[List[ndarray]]]) -> List[List[ndarray]]:
    theta_avg: List[List[ndarray]] = thetas[0]
    for theta in thetas[1:]:
        for layer_index, layer in enumerate(theta):
            for weight_index, weights in enumerate(layer):
                theta_avg[layer_index][weight_index] += weights
    for layer in theta_avg:
        for weights in layer:
            weights /= len(thetas)
    return theta_avg


def _get_model_params(model: tf.keras.Model) -> List[List[ndarray]]:
    theta = []
    for layer in model.layers:
        theta.append(layer.get_weights())
    return theta


def _set_model_params(model: tf.keras.Model, theta: List[List[ndarray]]):
    for layer, layer_weights in zip(model.layers, theta):
        layer.set_weights(layer_weights)
