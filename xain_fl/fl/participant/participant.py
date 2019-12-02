"""Class Participant handles local training in federated learning using its own
data partition to refine the global model.
"""
import os
from typing import Dict, List, Tuple

import numpy as np
import tensorflow as tf

from xain_fl.datasets import prep
from xain_fl.logger import get_logger
from xain_fl.types import History, Metrics, Partition, Theta, VolumeByClass

from .model_provider import ModelProvider

logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


class Participant:
    # pylint: disable-msg=too-many-arguments
    # pylint: disable=too-many-instance-attributes
    def __init__(
        self,
        cid: int,
        model_provider: ModelProvider,
        xy_train: Partition,
        xy_val: Partition,
        num_classes: int,
        batch_size: int,
        use_lr_fn: bool = True,
    ) -> None:
        """Initializes the participant

        Args:
            cid (int): Participant identifier
            model_provider (ModelProvider)
            xy_train (Partition): Training data
            xy_val (Partition): Validation data
            num_classes (int): Number of classes (classification)
            batch_size (int)
            use_lr_fn (bool = True): Optional boolean to enable learning rate function,
                defaults to True
        """
        assert xy_train[0].shape[0] == xy_train[1].shape[0]
        assert xy_val[0].shape[0] == xy_val[1].shape[0]
        self.cid = cid
        self.model_provider = model_provider
        self.num_classes: int = num_classes
        self.batch_size: int = batch_size
        self.use_lr_fn: bool = use_lr_fn
        self.num_examples = xy_train[0].shape[0]
        # Training set
        self.xy_train = xy_train
        self.steps_train: int = int(xy_train[0].shape[0] / batch_size)
        # Validation set
        self.xy_val = xy_val
        self.steps_val: int = 1

    def train_round(
        self, theta: Theta, epochs: int, epoch_base: int
    ) -> Tuple[Tuple[Theta, int], History, Dict]:
        """Performs the responsibilities of a single participant in a single round
            of federated learning.

        Args:
            theta (Theta): Current global model
            epochs (int): Number of local training epochs
            epoch_base (int): Number of globally performed epochs

        Returns:
            Tuple[Tuple[Theta, int], History, Dict]: Theta prime, local training history,
                and optimizer configs
        """
        logger.info(
            "Participant %s: train_round START (epoch_base=%s)", self.cid, epoch_base
        )
        model = self.model_provider.init_model(epoch_base=epoch_base)  # type:ignore
        model.set_weights(theta)

        callbacks: List = []
        if self.use_lr_fn:
            lr_fn = self.model_provider.init_lr_fn(epoch_base=epoch_base)  # type:ignore
            callback_lr = tf.keras.callbacks.LearningRateScheduler(lr_fn)
            callbacks = [callback_lr]

        hist: History = self._fit(model, epochs, callbacks)
        theta_prime = model.get_weights()
        opt_config = model.optimizer.get_config()
        opt_config = _convert_numpy_types(opt_config)
        logger.info("Participant %s: train_round FINISH", self.cid)
        return (theta_prime, self.num_examples), hist, opt_config

    def _fit(self, model: tf.keras.Model, epochs: int, callbacks: List) -> History:
        ds_train = prep.init_ds_train(self.xy_train, self.num_classes, self.batch_size)
        ds_val = prep.init_ds_val(self.xy_val, self.num_classes)

        callback_logging = LoggingCallback(str(self.cid), logger.info)
        callbacks.append(callback_logging)

        hist = model.fit(
            ds_train,
            epochs=epochs,
            validation_data=ds_val,
            callbacks=callbacks,
            shuffle=False,  # Shuffling is handled via tf.data.Dataset
            steps_per_epoch=self.steps_train,
            validation_steps=self.steps_val,
            verbose=0,
        )
        return _cast_to_float(hist.history)

    def evaluate(self, theta: Theta, xy_test: Partition) -> Tuple[float, float]:
        """Evaluate the local model using the provided validation data.

        Args:
            theta (Theta)
            xy_test (Partition)

        Returns:
            Tuple[float, float]: Loss and accuracy
        """
        model = self.model_provider.init_model()
        model.set_weights(theta)
        ds_val = prep.init_ds_val(xy_test)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = model.evaluate(ds_val, steps=1, verbose=0)
        return loss, accuracy

    def metrics(self) -> Metrics:
        """Provides training metrics.

        Returns:
            (int, VolumeByClass)
        """
        vol_by_class = _xy_train_volume_by_class(self.num_classes, self.xy_train)
        return (self.cid, vol_by_class)


def _xy_train_volume_by_class(num_classes: int, xy_train) -> VolumeByClass:
    counts = [0] * num_classes
    _, y = xy_train
    classes, counts_actual = np.unique(y, return_counts=True)

    for i_ca, c in enumerate(classes):
        # Cast explicitly to int so its later JSON serializable
        # as other we will get a list of np objects of type int64
        counts[c] = int(counts_actual[i_ca])

    return counts


def _cast_to_float(hist) -> History:
    for key in hist:
        for index, number in enumerate(hist[key]):
            hist[key][index] = float(number)
    return hist


def _convert_numpy_types(opt_config: Dict) -> Dict:
    for key in opt_config:
        if isinstance(opt_config[key], np.float32):
            opt_config[key] = opt_config[key].item()
    return opt_config


class LoggingCallback(tf.keras.callbacks.Callback):
    def __init__(self, cid: str, print_fn):
        tf.keras.callbacks.Callback.__init__(self)
        self.cid = cid
        self.print_fn = print_fn

    def on_epoch_end(self, epoch, logs=None):
        self.print_fn(f"CID {self.cid} epoch {epoch}")
