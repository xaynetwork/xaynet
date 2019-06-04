from typing import Tuple
import numpy as np
from numpy import ndarray
import tensorflow as tf
from autofl.mnist_f import mnist_f
from autofl.fedml import net

PARTICIPANTS = 3


def main():
    # Load data
    x_splits, y_splits, x_test, y_test = mnist_f.load_splits(num_splits=PARTICIPANTS)
    print(len(x_splits))
    print(len(y_splits))
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


class Participant:
    def __init__(
        self, model: tf.keras.Model, x_split: ndarray, y_split: ndarray
    ) -> None:
        assert x_split.shape[0] == y_split.shape[0]
        self.model = model
        self.x_split = x_split
        self.y_split = y_split
        self.history = None

    def train(self, epochs: int):
        x_train = self.x_split / 255.0
        y_train = self.y_split
        self.history = self.model.fit(x_train, y_train, epochs=epochs)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy
