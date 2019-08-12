from typing import Dict, List, Tuple

import numpy as np
import tensorflow as tf

from autofl.datasets import prep
from autofl.net import orig_cnn_compiled
from autofl.types import FederatedDatasetPartition, KerasWeights

NUM_CLASSES = 10
BATCH_SIZE = 64


class Participant:
    # pylint: disable-msg=too-many-arguments
    def __init__(
        self,
        model: tf.keras.Model,
        xy_train: Tuple[np.ndarray, np.ndarray],
        xy_val: Tuple[np.ndarray, np.ndarray],
        num_classes: int = NUM_CLASSES,
        batch_size: int = BATCH_SIZE,
    ) -> None:
        assert xy_train[0].shape[0] == xy_train[1].shape[0]
        assert xy_val[0].shape[0] == xy_val[1].shape[0]
        self.model = model
        # Training set
        self.ds_train = prep.init_ds_train(xy_train, num_classes, batch_size)
        self.steps_train = int(xy_train[0].shape[0] / BATCH_SIZE)
        # Validation set
        self.ds_val = prep.init_ds_val(xy_val, num_classes)
        self.steps_val = 1

    def train_round(self, theta: KerasWeights, epochs) -> KerasWeights:
        self.model.set_weights(theta)
        _ = self._train(epochs)
        theta_prime = self.model.get_weights()
        return theta_prime

    def _train(self, epochs: int) -> Dict[str, List[float]]:
        hist = self.model.fit(
            self.ds_train,
            epochs=epochs,
            validation_data=self.ds_val,
            shuffle=False,  # Shuffling is handled via tf.data.Dataset
            steps_per_epoch=self.steps_train,
            validation_steps=self.steps_val,
            verbose=2,
        )
        return cast_to_float(hist.history)

    def evaluate(self, xy_test: Tuple[np.ndarray, np.ndarray]) -> Tuple[float, float]:
        ds_val = prep.init_ds_val(xy_test)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return loss, accuracy

    def replace_model(self, model: tf.keras.Model) -> None:
        self.model = model


def cast_to_float(hist):
    for key in hist:
        for index, number in enumerate(hist[key]):
            hist[key][index] = float(number)
    return hist


def init_participants(
    xy_partitions: List[FederatedDatasetPartition], xy_val: FederatedDatasetPartition
) -> List[Participant]:
    participants: List[Participant] = []
    for xy_train in xy_partitions:
        model = orig_cnn_compiled()  # FIXME refactor
        participant = Participant(model, xy_train, xy_val)
        participants.append(participant)
    return participants
