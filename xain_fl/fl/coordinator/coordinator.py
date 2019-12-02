"""Class Coordinator orchestrates federated learning over a number of participants
using a selection strategy (implemented through Controller sub-class) and an aggregation
method (implemented through Aggregator sub-class).
"""
import concurrent.futures
import os
from pathlib import Path
from typing import Callable, Dict, List, Optional, Tuple

import tensorflow as tf
from absl import flags

from xain_fl.datasets import prep
from xain_fl.fl.logging.logging import create_summary_writer, write_summaries
from xain_fl.fl.participant import ModelProvider, Participant
from xain_fl.logger import get_logger
from xain_fl.types import History, Metrics, Partition, Theta

from .aggregate import Aggregator, FederatedAveragingAgg

FLAGS = flags.FLAGS

logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


class Coordinator:
    """Central class of federated learning."""

    # pylint: disable-msg=too-many-arguments
    # pylint: disable=too-many-instance-attributes
    def __init__(
        self,
        controller,
        model_provider: ModelProvider,
        participants: List[Participant],
        C: float,
        E: int,
        xy_val: Partition,
        aggregator: Optional[Aggregator] = None,
    ) -> None:
        """Initializes coordinator.

        Args:
            controller (Controller): Required selection strategy
            model_provider (ModelProvider)
            participants (List[Participant])
            C (float): Fraction of participants selected in each round
            E (int): Number of epochs in each round
            xy_val (Partition): Validation data partition
            aggregator (Optional[Aggregator] = None): Optional aggreation method, defaults
                to FederatedAveragingAgg
        """
        self.controller = controller
        self.model = model_provider.init_model()
        self.participants = participants
        self.C = C
        self.E = E
        self.xy_val = xy_val
        self.aggregator = aggregator if aggregator else FederatedAveragingAgg()
        self.epoch = 0  # Count training epochs

    # Common initialization happens implicitly: By updating the participant weights to
    # match the coordinator weights ahead of every training round we achieve common
    # initialization.
    def fit(
        self, num_rounds: int
    ) -> Tuple[History, List[List[History]], List[List[Dict]], List[List[Metrics]]]:
        """Performs federated learning for a given number of rounds.

        Args:
            num_rounds (int): Number of rounds to run the federated learning

        Returns:
            Tuple[History, List[List[History]], List[List[Dict]], List[List[Metrics]]]
        """
        # Initialize history; history coordinator
        hist_co: History = {"val_loss": [], "val_acc": []}
        # Train rounds; training history of selected participants
        hist_ps: List[List[History]] = []
        # History of optimizer configs in each round
        hist_opt_configs: List[List[Dict]] = []
        # History of participant metrics in each round
        hist_metrics: List[List[Metrics]] = []

        # Defining log directory and file writer for tensorboard logging
        val_log_dir: str = str(
            Path(FLAGS.output_dir).joinpath("tensorboard/coordinator")
        )
        summary_writer = create_summary_writer(logdir=val_log_dir)

        for r in range(num_rounds):
            # Determine who participates in this round
            num_indices = _abs_C(self.C, self.num_participants())
            indices = self.controller.indices(num_indices)
            msg = f"Round {r+1}/{num_rounds}: Participants {indices}"
            logger.info(msg)

            # Train
            histories, opt_configs, train_metrics = self.fit_round(indices, self.E)
            hist_ps.append(histories)
            hist_opt_configs.append(opt_configs)
            hist_metrics.append(train_metrics)

            # Evaluate
            val_loss, val_acc = self.evaluate(self.xy_val)
            # Writing validation loss and accuracy into summary
            write_summaries(
                summary_writer=summary_writer,
                val_acc=val_acc,
                val_loss=val_loss,
                train_round=r,
            )
            hist_co["val_loss"].append(val_loss)
            hist_co["val_acc"].append(val_acc)

        logger.info("TensorBoard coordinator validation logs saved: %s", val_log_dir)
        logger.info(
            'Detailed analysis: call "tensorboard --logdir %s" from the \
            console and open "localhost:6006" in a browser',
            val_log_dir,
        )

        return hist_co, hist_ps, hist_opt_configs, hist_metrics

    def fit_round(
        self, indices: List[int], E: int
    ) -> Tuple[List[History], List[Dict], List[Metrics]]:
        """Performs a single round of federated learning.

        Args:
            indices (List[int]): Selected indices for round.
            E (int): Number of local epochs to train.

        Returns:
            Tuple[List[History], List[Dict], List[Metrics]]
        """
        theta = self.model.get_weights()
        participants = [self.participants[i] for i in indices]
        # Collect training results from the participants of this round
        theta_updates, histories, opt_configs, train_metrics = self.train_local_concurrently(
            theta, participants, E
        )
        # Aggregate training results
        theta_prime = self.aggregator.aggregate(theta_updates)
        # Update own model parameters
        self.model.set_weights(theta_prime)
        self.epoch += E
        return histories, opt_configs, train_metrics

    def train_local_sequentially(
        self, theta: Theta, participants: List[Participant], E: int
    ) -> Tuple[List[Tuple[Theta, int]], List[History], List[Dict], List[Metrics]]:
        """Train on each participant sequentially.

        Args:
            theta (Theta): Current global model parameters
            participants (List[Participant]): Selected participants
            E (int): Number of local training epochs

        Returns:
            Tuple[List[Tuple[Theta, int]], List[History], List[Dict], List[Metrics]]: Theta
                primes, local training histories, optimizer configs, training metrics
        """
        theta_updates: List[Tuple[Theta, int]] = []
        histories: List[History] = []
        opt_configs: List[Dict] = []
        train_metrics: List[Metrics] = []
        for participant in participants:
            # Train one round on this particular participant:
            # - Push current model parameters to this participant
            # - Train for a number of epochs
            # - Pull updated model parameters from participant
            theta_update, hist, opt_config = participant.train_round(
                theta, epochs=E, epoch_base=self.epoch
            )
            metrics = participant.metrics()
            theta_updates.append(theta_update)
            histories.append(hist)
            opt_configs.append(opt_config)
            train_metrics.append(metrics)
        return theta_updates, histories, opt_configs, train_metrics

    def train_local_concurrently(
        self, theta: Theta, participants: List[Participant], E: int
    ) -> Tuple[List[Tuple[Theta, int]], List[History], List[Dict], List[Metrics]]:
        """Train on each participant concurrently.

        Args:
            theta (Theta): Current global model parameters
            participants (List[Participant]): Selected participants
            E (int): Number of local training epochs

        Returns:
            Tuple[List[Tuple[Theta, int]], List[History], List[Dict], List[Metrics]]: Theta
                primes, local training histories, optimizer configs, training metrics
        """
        theta_updates: List[Tuple[Theta, int]] = []
        histories: List[History] = []
        opt_configs: List[Dict] = []
        train_metrics: List[Metrics] = []
        # Wait for all futures to complete
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_results = [
                executor.submit(_train_local, p, theta, E, self.epoch)
                for p in participants
            ]
            concurrent.futures.wait(future_results)
            for future in future_results:
                theta_update, hist, opt_config, metrics = future.result()
                theta_updates.append(theta_update)
                histories.append(hist)
                opt_configs.append(opt_config)
                train_metrics.append(metrics)
        return theta_updates, histories, opt_configs, train_metrics

    def evaluate(self, xy_val: Partition) -> Tuple[float, float]:
        """Evaluate the global model using the provided validation data.

        Args:
            xy_val (Partition)

        Returns:
            Tuple[float, float]: Loss and accuracy
        """
        ds_val = prep.init_ds_val(xy_val)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        loss, accuracy = self.model.evaluate(ds_val, steps=1)
        return float(loss), float(accuracy)

    def num_participants(self) -> int:
        """Returns number of participants

        Returns:
            int
        """
        return len(self.participants)


def _train_local(
    p: Participant, theta: Theta, epochs: int, epoch_base: int
) -> Tuple[Tuple[Theta, int], History, Dict, Metrics]:
    theta_update, history, opt_config = p.train_round(
        theta, epochs=epochs, epoch_base=epoch_base
    )
    metrics = p.metrics()
    return theta_update, history, opt_config, metrics


def _abs_C(C: float, num_participants: int) -> int:
    return int(min(num_participants, max(1, C * num_participants)))


def _create_evalueate_fn(
    orig_model: tf.keras.Model, xy_val: Partition
) -> Callable[[Theta], Tuple[float, float]]:
    ds_val = prep.init_ds_val(xy_val)
    model = tf.keras.models.clone_model(orig_model)
    # FIXME refactor model compilation
    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )

    def fn(theta: Theta) -> Tuple[float, float]:
        model.set_weights(theta)
        # Assume the validation `tf.data.Dataset` to yield exactly one batch containing
        # all examples in the validation set
        return model.evaluate(ds_val, steps=1)

    return fn
