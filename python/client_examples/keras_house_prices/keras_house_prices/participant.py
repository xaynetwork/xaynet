"""Tensorflow Keras regression test case"""

import argparse
from io import BytesIO
import logging
import os
import random
from typing import List, Optional, Tuple

import numpy as np
import pandas as pd
from tabulate import tabulate
from xain_sdk import ParticipantABC, configure_logging, run_participant

from keras_house_prices.regressor import Regressor

LOG = logging.getLogger(__name__)


class Participant(  # pylint: disable=too-few-public-methods,too-many-instance-attributes
    ParticipantABC
):
    """An example of a Keras implementation of a participant for federated
    learning.

    The attributes for the model and the datasets are only for
    convenience, they might as well be loaded elsewhere.

    Attributes:

        regressor: The model to be trained.
        trainset_x: A dataset for training.
        trainset_y: Labels for training.
        testset_x: A dataset for test.
        testset_y: Labels for test.
        number_samples: The number of samples in the training dataset.
        performance_metrics: metrics collected after each round of training

    """

    def __init__(self, dataset_dir: str) -> None:
        """Initialize a custom participant.
        """
        super(Participant, self).__init__()
        self.load_random_dataset(dataset_dir)
        self.regressor = Regressor(len(self.trainset_x.columns))
        self.performance_metrics: List[Tuple[float, float]] = []

    def load_random_dataset(self, dataset_dir: str) -> None:
        """Load a random dataset from the data directory

        """
        i = random.randrange(0, 10, 1)

        LOG.info("Train on sample number %d", i)
        trainset_file_path = os.path.join(
            dataset_dir, "split_data", f"data_part_{i}.csv"
        )

        trainset = pd.read_csv(trainset_file_path, index_col=None)
        self.trainset_x = trainset.drop("Y", axis=1)
        self.trainset_y = trainset["Y"]
        self.number_of_samples = len(trainset)

        testset_file_path = os.path.join(dataset_dir, "test.csv")
        testset = pd.read_csv(testset_file_path, index_col=None)
        testset_x = testset.drop("Y", axis=1)
        self.testset_x: pd.DataFrame = testset_x.drop(testset_x.columns[0], axis=1)
        self.testset_y = testset["Y"]

    def train_round(
        self, training_input: Optional[np.ndarray]
    ) -> Tuple[np.ndarray, int]:
        """Train a model in a federated learning round.

        A model is given in terms of its weights and the model is
        trained on the participant's dataset for a number of
        epochs. The weights of the updated model are returned in
        combination with the number of samples of the train dataset.

        Args:

            weights: The weights of the model to be trained.

        Returns:

            The updated model weights and the number of training samples.

        """
        if training_input is None:
            # This is the first round: the coordinator doesn't have a
            # global model yet, so we need to initialize the weights
            self.regressor = Regressor(len(self.trainset_x.columns))
            return (self.regressor.get_weights(), 0)

        weights = training_input
        # FIXME: what should this be?
        epochs = 10
        self.regressor.set_weights(weights)
        self.regressor.train_n_epochs(epochs, self.trainset_x, self.trainset_y)

        loss: float
        r_squared: float
        loss, r_squared = self.regressor.evaluate_on_test(
            self.testset_x, self.testset_y
        )
        LOG.info("loss = %f, R² = %f", loss, r_squared)
        self.performance_metrics.append((loss, r_squared))

        return (self.regressor.get_weights(), self.number_of_samples)

    def deserialize_training_input(self, data: bytes) -> Optional[np.ndarray]:
        if not data:
            return None

        reader = BytesIO(data)
        return np.load(reader, allow_pickle=False)

    def serialize_training_result(
        self, training_result: Tuple[np.ndarray, int]
    ) -> bytes:
        (weights, number_of_samples) = training_result

        writer = BytesIO()
        writer.write(number_of_samples.to_bytes(4, byteorder="big"))
        np.save(writer, weights, allow_pickle=False)
        return writer.getbuffer()[:]


def main() -> None:
    """Entry point to start a participant."""
    parser = argparse.ArgumentParser(description="Prepare data for regression")
    parser.add_argument(
        "--write-performance-metrics",
        type=str,
        help="Path to a file where the participant will write performance metrics",
    )
    parser.add_argument(
        "--data-directory",
        type=str,
        help="path to the directory that contains the data",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="Log the HTTP requests",
    )
    parser.add_argument(
        "--coordinator-url", type=str, required=True, help="URL of the coordinator",
    )
    parser.add_argument(
        "--heartbeat-period", type=float, default=1, help="Heartbeat period in seconds",
    )
    args = parser.parse_args()

    # pylint: disable=invalid-name
    logging.basicConfig(
        format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
        level=logging.DEBUG,
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    if args.verbose:
        configure_logging(log_http_requests=True)
    else:
        configure_logging(log_http_requests=False)

    participant = Participant(args.data_directory)
    run_participant(
        participant, args.coordinator_url, heartbeat_period=args.heartbeat_period
    )

    table = tabulate(participant.performance_metrics, headers=["Loss", "R²"])
    if args.write_performance_metrics:
        with open(args.write_performance_metrics, "w") as f:
            f.write(table)
    else:
        print(table)


if __name__ == "__main__":
    main()
