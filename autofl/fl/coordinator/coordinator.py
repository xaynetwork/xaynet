import concurrent.futures
from typing import Callable, Dict, List, Optional, Tuple

import tensorflow as tf
from absl import logging
from numpy import ndarray

from autofl.datasets import prep
from autofl.fl.participant import ModelProvider, Participant
from autofl.types import KerasHistory, KerasWeights

from .aggregate import Aggregator, WeightedAverageAgg


class Coordinator:
    # pylint: disable-msg=too-many-arguments
    def __init__(
        self,
        controller,
        model_provider: ModelProvider,
        participants: List[Participant],
        C: float,
        E: int,
        xy_val: Tuple[ndarray, ndarray],
        aggregator: Optional[Aggregator] = None,
    ) -> None:
        self.controller = controller
        self.model = model_provider.init_model()
        self.participants = participants
        self.C = C
        self.E = E
        self.xy_val = xy_val
        self.aggregator = aggregator if aggregator else WeightedAverageAgg()

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def fit(self, num_rounds: int) -> Tuple[KerasHistory, List[List[KerasHistory]]]:
        # Evaluate initial model and initialize history
        loss, acc = self.evaluate(self.xy_val)
        hist_co: KerasHistory = {"val_loss": [loss], "val_acc": [acc]}
        # Train rounds
        hist_ps: List[List[KerasHistory]] = []
        for r in range(num_rounds):
            # Determine who participates in this round
            num_indices = abs_C(self.C, self.num_participants())
            indices = self.controller.indices(num_indices)
            msg = "Round {}/{}: Participants {}".format(r + 1, num_rounds, indices)
            logging.info(msg)
            # Train
            histories = self.fit_round(indices)  # TODO use return value (i.e. history)
            hist_ps.append(histories)
            # Evaluate
            val_loss, val_acc = self.evaluate(self.xy_val)
            hist_co["val_loss"].append(val_loss)
            hist_co["val_acc"].append(val_acc)
        return hist_co, hist_ps

    def fit_round(self, indices: List[int]) -> List[KerasHistory]:
        theta = self.model.get_weights()
        participants = [self.participants[i] for i in indices]
        # Collect training results from the participants of this round
        theta_updates, histories = self.train_local_concurrently(theta, participants)
        # Aggregate training results
        theta_prime = self.aggregator.aggregate(theta_updates)
        # Update own model parameters
        self.model.set_weights(theta_prime)
        return histories

    def train_local_sequentially(
        self, theta: KerasWeights, participants: List[Participant]
    ) -> Tuple[List[KerasWeights], List[KerasHistory]]:
        """Train on each participant sequentially"""
        theta_updates = []
        histories: List[KerasHistory] = []
        for participant in participants:
            # Train one round on this particular participant:
            # - Push current model parameters to this participant
            # - Train for a number of epochs
            # - Pull updated model parameters from participant
            theta_update, hist = participant.train_round(theta, epochs=self.E)
            theta_updates.append(theta_update)
            histories.append(hist)
        return theta_updates, histories

    def train_local_concurrently(
        self, theta: KerasWeights, participants: List[Participant]
    ) -> Tuple[List[KerasWeights], List[KerasHistory]]:
        """Train on each participant concurrently"""
        theta_updates = []
        histories: List[KerasHistory] = []
        # Wait for all futures to complete
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_results = [
                executor.submit(train_local, p, theta, self.E) for p in participants
            ]
            concurrent.futures.wait(future_results)
            for future in future_results:
                theta_update, hist = future.result()
                theta_updates.append(theta_update)
                histories.append(hist)
        return theta_updates, histories

    def evaluate(self, xy_val: Tuple[ndarray, ndarray]) -> Tuple[float, float]:
        ds_val = prep.init_ds_val(xy_val)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return float(loss), float(accuracy)

    def num_participants(self) -> int:
        return len(self.participants)


def train_local(p: Participant, theta: KerasWeights, epochs: int) -> KerasWeights:
    theta_prime = p.train_round(theta, epochs=epochs)
    return theta_prime


def abs_C(C: float, num_participants: int) -> int:
    return int(min(num_participants, max(1, C * num_participants)))


def create_evalueate_fn(
    orig_model: tf.keras.Model, xy_val: Tuple[ndarray, ndarray]
) -> Callable[[KerasWeights], Tuple[float, float]]:
    ds_val = prep.init_ds_val(xy_val)
    model = tf.keras.models.clone_model(orig_model)
    # FIXME refactor model compilation
    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )

    def fn(theta: KerasWeights) -> Tuple[float, float]:
        model.set_weights(theta)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        return model.evaluate(ds_val, steps=1)

    return fn
