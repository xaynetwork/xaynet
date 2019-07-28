from typing import Any, Callable, List, Tuple

import tensorflow as tf
from numpy import ndarray

from ..datasets import prep
from .ops import get_model_params, set_model_params
from .participant import Participant


class Coordinator:
    def __init__(
        self, controller, model: tf.keras.Model, participants: List[Participant]
    ) -> None:
        self.controller = controller
        self.model = model
        self.participants = participants

    # TODO remove or refactor: only needed for FedNasEnv
    def replace_model(self, model_fn: Callable[..., tf.keras.Model]) -> None:
        self.model = model_fn()
        for p in self.participants:
            model = model_fn()
            p.replace_model(model)

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def fit(self, num_rounds: int):
        history_updates: List[List[Any]] = []
        for training_round in range(num_rounds):
            # Determine who participates in this round
            indices = self.controller.indices()
            print("\nRound", str(training_round + 1), "- participants", indices)
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
        theta_prime = self.controller.aggregate(thetas)
        # Update own model parameters
        set_model_params(self.model, theta_prime)
        # Report progress
        return histories

    def _single_step(self, random_index: int) -> Tuple[List[List[ndarray]], Any]:
        participant = self.participants[random_index]
        # Train one round on this particular participant:
        # - Push current model parameters to this participant
        # - Train for a number of epochs
        # - Pull updated model parameters from participant
        theta = get_model_params(self.model)
        theta_prime, history = participant.train_round(theta, epochs=1)
        return theta_prime, history

    def evaluate(self, xy_val: Tuple[ndarray, ndarray]) -> Tuple[float, float]:
        ds_val = prep.init_ds_val(xy_val[0], xy_val[1])
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return loss, accuracy

    def num_participants(self) -> int:
        return len(self.participants)


def abs_C(C: float, num_participants: int):
    return min(num_participants, max(1, C * num_participants))


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
