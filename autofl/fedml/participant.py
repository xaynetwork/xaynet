from typing import List, Tuple

import tensorflow as tf
from numpy import ndarray

from ..data import prep
from . import net, ops


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
        ops.set_model_params(self.model, theta)

    def retrieve_model_parameters(self) -> List[List[ndarray]]:
        return ops.get_model_params(self.model)

    def train(self, epochs: int):
        self.history = self.model.fit(self.dataset, epochs=epochs, steps_per_epoch=8)

    def evaluate(self, x_test: ndarray, y_test: ndarray) -> Tuple[float, float]:
        # FIXME use Dataset
        x_test = x_test / 255.0
        loss, accuracy = self.model.evaluate(x_test, y_test)
        return loss, accuracy


def init_participants(xy_splits) -> List[Participant]:
    participants = []
    for x_split, y_split in xy_splits:
        model = net.cnn()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    return participants
