from typing import Any, List, Tuple

import numpy as np
import tensorflow as tf

from autofl.datasets import prep
from autofl.net import orig_cnn_compiled

from . import ops

BATCH_SIZE = 64


class Participant:
    def __init__(
        self,
        model: tf.keras.Model,
        xy_train: Tuple[np.ndarray, np.ndarray],
        xy_val: Tuple[np.ndarray, np.ndarray],
    ) -> None:
        assert xy_train[0].shape[0] == xy_train[1].shape[0]
        assert xy_val[0].shape[0] == xy_val[1].shape[0]
        self.model = model
        self.ds_train = prep.init_dataset(xy_train[0], xy_train[1])
        self.ds_val = prep.init_dataset(xy_val[0], xy_val[1])
        self.steps_train = int(xy_train[0].shape[0] / BATCH_SIZE)
        self.steps_val = int(xy_val[0].shape[0] / BATCH_SIZE)

    def train_round(
        self, theta: List[List[np.ndarray]], epochs
    ) -> Tuple[List[List[np.ndarray]], Any]:
        self.update_model_parameters(theta)
        history = self.train(epochs)
        theta_prime = self.retrieve_model_parameters()
        return theta_prime, history

    def update_model_parameters(self, theta: List[List[np.ndarray]]) -> None:
        ops.set_model_params(self.model, theta)

    def retrieve_model_parameters(self) -> List[List[np.ndarray]]:
        return ops.get_model_params(self.model)

    def train(self, epochs: int) -> Any:
        history = self.model.fit(
            self.ds_train,
            epochs=epochs,
            validation_data=self.ds_val,
            shuffle=False,  # Shuffling is handled via tf.data.Dataset
            steps_per_epoch=self.steps_train,
            validation_steps=self.steps_val,
        )
        return history

    def evaluate(self, xy_test: Tuple[np.ndarray, np.ndarray]) -> Tuple[float, float]:
        ds_val = prep.init_validation_dataset(xy_test[0], xy_test[1])
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return loss, accuracy

    def replace_model(self, model: tf.keras.Model) -> None:
        self.model = model


def init_participants(xy_splits, xy_val) -> List[Participant]:
    participants = []
    for xy_train in xy_splits:
        model = orig_cnn_compiled()  # FIXME refactor
        participant = Participant(model, xy_train, xy_val)
        participants.append(participant)
    return participants
