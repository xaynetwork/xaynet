from typing import Any, Callable, List, Optional, Tuple

import tensorflow as tf
from absl import logging
from numpy import ndarray

from autofl.datasets import prep
from autofl.types import KerasWeights

from .aggregate import Aggregator, WeightedAverageAgg
from .participant import Participant


class Coordinator:
    # pylint: disable-msg=too-many-arguments
    def __init__(
        self,
        controller,
        model: tf.keras.Model,
        participants: List[Participant],
        C: float,
        E: int = 1,
        aggregator: Optional[Aggregator] = None,
    ) -> None:
        self.controller = controller
        self.model = model
        self.participants = participants
        self.C = C
        self.E = E
        self.aggregator = aggregator if aggregator else WeightedAverageAgg()

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def fit(self, num_rounds: int):
        history_updates: List[List[Any]] = []
        for training_round in range(num_rounds):
            # Determine who participates in this round
            num_indices = abs_C(self.C, self.num_participants())
            indices = self.controller.indices(num_indices)
            logging.info(
                "\nRound {}/{}: Participants {}".format(
                    training_round + 1, num_rounds, indices
                )
            )
            histories = self.fit_round(indices)
            history_updates.append(histories)
        # Return aggregated histories
        return aggregate_histories(history_updates)

    def fit_round(self, indices: List[int]):
        # Collect training results from the participants of this round
        thetas = []
        histories = []
        for index in indices:
            theta, history = self._single_step(index)
            thetas.append(theta)
            histories.append(history)
        # Aggregate training results
        theta_prime = self.aggregator.aggregate(thetas)
        # Update own model parameters
        self.model.set_weights(theta_prime)
        # Report progress
        return histories

    def _single_step(self, random_index: int) -> Tuple[KerasWeights, Any]:
        participant = self.participants[random_index]
        # Train one round on this particular participant:
        # - Push current model parameters to this participant
        # - Train for a number of epochs
        # - Pull updated model parameters from participant
        theta = self.model.get_weights()
        theta_prime, history = participant.train_round(theta, epochs=self.E)
        return theta_prime, history

    def evaluate(self, xy_val: Tuple[ndarray, ndarray]) -> Tuple[float, float]:
        ds_val = prep.init_ds_val(xy_val)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return loss, accuracy

    def num_participants(self) -> int:
        return len(self.participants)


def abs_C(C: float, num_participants: int) -> int:
    return int(min(num_participants, max(1, C * num_participants)))


def aggregate_histories(history_updates):
    history = history_updates[0][0]
    for histories in history_updates[1:]:
        h0 = history.history
        h1 = histories[0].history
        h_prime = history_update(h0, h1)
        history.history = h_prime
    return history


def history_update(h0, h1):
    for k in h1.keys():
        vals = h1[k]
        h0[k] = h0[k] + vals
    return h0


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
