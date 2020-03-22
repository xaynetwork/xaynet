"""Tensorflow Keras regression test case"""

import argparse
import logging
import os
import pickle
import random
from typing import List, Tuple, TypeVar

import numpy as np
import pandas as pd
from tabulate import tabulate
from xain_sdk import (
    ParticipantABC,
    TrainingInputABC,
    TrainingResultABC,
    configure_logging,
    run_participant,
)

from keras_house_prices.regressor import Regressor

LOG = logging.getLogger(__name__)

# pylint: disable=invalid-name
T = TypeVar("T", bound="TrainingInput")


class TrainingInput(TrainingInputABC):
    def __init__(self, weights: np.ndarray):
        self.weights = weights

    @staticmethod
    def frombytes(data: bytes) -> T:
        weights = pickle.loads(data)
        return TrainingInput(weights)

    def is_initialization_round(self) -> bool:
        return self.weights is None


class TrainingResult(TrainingResultABC):
    def __init__(self, weights: np.ndarray, number_of_samples: int):
        self.weights = weights
        self.number_of_samples = number_of_samples

    def tobytes(self) -> bytes:
        data = self.number_of_samples.to_bytes(4, byteorder="big")
        return data + pickle.dumps(self.weights)


class Participant(  # pylint: disable=too-few-public-methods,too-many-instance-attributes
    ParticipantABC
):
    """An example of a Keras implementation of a participant for federated
    learning.

    The attributes for the model and the datasets are only for
    convenience, they might as well be loaded elsewhere.

    Attributes:

        model: The model to be trained.
        trainset_x: A dataset for training.
        trainset_y: Labels for training.
        testset_x: A dataset for test.
        testset_y: Labels for test.
        number_samples: The number of samples in the training dataset.
        flattened: flattened vector of models weights
        shape: CNN model architecture
        indices: indices of split points in the flattened vector
        performance_metrics: metrics collected after each round of training

    """

    def __init__(self, dataset_dir: str) -> None:
        """Initialize the custom participant.

        The model and the datasets are defined here only for
        convenience, they might as well be loaded in the
        ``train_round()`` method on the fly. Due to the nature of this
        example, the model is a simple dense neural network and the
        datasets are randomly generated.

        """

        super(Participant, self).__init__()
        # define or load a model to be trained
        i = random.randrange(0, 10, 1)

        LOG.info("Train on sample number %d", i)
        trainset_filename = f"data_part_{i}.csv"
        testset_filename = "test.csv"
        trainset_file_path = os.path.join(dataset_dir, "split_data", trainset_filename)
        testset_file_path = os.path.join(dataset_dir, testset_filename)

        trainset = pd.read_csv(trainset_file_path, index_col=None)
        testset = pd.read_csv(testset_file_path, index_col=None)
        self.trainset_x = trainset.drop("Y", axis=1)
        self.trainset_y = trainset["Y"]
        self.testset_x = testset.drop("Y", axis=1)
        self.testset_x = self.testset_x.drop(self.testset_x.columns[0], axis=1)
        self.testset_y = testset["Y"]
        self.model = Regressor(len(self.trainset_x.columns))
        self.shapes: List[Tuple[int, ...]] = self.get_tensorflow_shapes()
        self.flattened: np.ndarray = self.get_tensorflow_weights()
        self.number_samples = len(trainset)
        self.performance_metrics: List[Tuple[float, float]] = []

    def init_weights(self) -> TrainingResult:
        """Initialize the weights of a model.

        The model weights are freshly initialized according to the
        participant's model definition and are returned without
        training.

        Returns:

            The newly initialized model weights.

        """

        self.model = Regressor(len(self.trainset_x.columns))
        self.flattened = self.get_tensorflow_weights()
        return TrainingResult(self.flattened, 0)

    def train_round(self, training_input: TrainingInput) -> TrainingResult:
        """Train a model in a federated learning round.

        A model is given in terms of its weights and the model is
        trained on the participant's dataset for a number of
        epochs. The weights of the updated model are returned in
        combination with the number of samples of the train dataset.

        Args:

            training_input: The weights of the model to be trained.

        Returns:

            The updated model weights and the number of training samples.

        """

        # FIXME: what should this be???
        epochs = 10
        self.set_tensorflow_weights(weights=training_input.weights, shapes=self.shapes)
        self.model.train_n_epochs(epochs, self.trainset_x, self.trainset_y)

        loss: float
        r_squared: float
        loss, r_squared = self.model.evaluate_on_test(self.testset_x, self.testset_y)
        LOG.info("loss = %f, R² = %f", loss, r_squared)
        self.performance_metrics.append((loss, r_squared))

        self.flattened = self.get_tensorflow_weights()
        return TrainingResult(self.flattened, self.number_samples)

    def deserialize_training_input(self, data: bytes) -> TrainingInput:
        if not data:
            return TrainingInput(None)
        return TrainingInput.frombytes(data)

    def get_tensorflow_shapes(self) -> List[Tuple[int, ...]]:
        return [weight.shape for weight in self.model.model.get_weights()]

    def get_tensorflow_weights(self) -> np.ndarray:
        return np.concatenate(self.model.model.get_weights(), axis=None)

    def set_tensorflow_weights(
        self, weights: np.ndarray, shapes: List[Tuple[int, ...]]
    ) -> None:
        # expand the flat weights
        indices: np.ndarray = np.cumsum([np.prod(shape) for shape in shapes])
        tensorflow_weights: List[np.ndarray] = np.split(
            weights, indices_or_sections=indices
        )
        tensorflow_weights = [
            np.reshape(weight, newshape=shape)
            for weight, shape in zip(tensorflow_weights, shapes)
        ]

        # apply the weights to the tensorflow model
        self.model.model.set_weights(tensorflow_weights)


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
        "--heartbeat-frequency",
        type=float,
        default=1,
        help="Frequency of the heartbeat in seconds",
    )
    args = parser.parse_args()

    # pylint: disable=invalid-name
    logging.basicConfig(
        format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
        level=logging.DEBUG,
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    if args.verbose:
        configure_logging(level=logging.DEBUG, log_http_requests=True)
    else:
        configure_logging(level=logging.INFO, log_http_requests=False)

    participant = Participant(args.data_directory)
    run_participant(
        participant, args.coordinator_url, heartbeat_frequency=args.heartbeat_frequency
    )

    table = tabulate(participant.performance_metrics, headers=["Loss", "R²"])
    if args.write_performance_metrics:
        with open(args.write_performance_metrics, "w") as f:
            f.write(table)
    else:
        print(table)


if __name__ == "__main__":
    main()
