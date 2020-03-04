import pickle
from typing import List, Tuple, TypeVar

# pylint: disable=import-error
import numpy as np
from numpy import ndarray
from tensorflow import Tensor
from tensorflow.data import Dataset
from tensorflow.keras import Input, Model
from tensorflow.keras.layers import Dense
from xain_sdk import (
    ParticipantABC,
    TrainingInputABC,
    TrainingResultABC,
    run_participant,
)

# pylint: disable=invalid-name
T = TypeVar("T", bound="TrainingInput")


class TrainingInput(TrainingInputABC):
    def __init__(self, weights: ndarray):
        self.weights = weights

    @staticmethod
    def frombytes(data: bytes) -> T:
        weights = pickle.loads(data)
        return TrainingInput(weights)

    def is_initialization_round(self) -> bool:
        return self.weights is None


class TrainingResult(TrainingResultABC):
    def __init__(self, weights: ndarray, number_of_samples: int):
        self.weights = weights
        self.number_of_samples = number_of_samples

    def tobytes(self) -> bytes:
        data = self.number_of_samples.to_bytes(4, byteorder="big")
        return data + pickle.dumps(self.weights)


class Participant(ParticipantABC):
    # pylint: disable=too-many-instance-attributes
    def __init__(
        self,
        features: int = 10,
        units: int = 6,
        categories: int = 2,
        train_samples: int = 80,
        val_samples: int = 10,
        test_samples: int = 10,
        batch_size: int = 8,
    ) -> None:
        self.features: int = features
        self.units: int = units
        self.categories: int = categories
        self.train_samples: int = train_samples
        self.val_samples: int = val_samples
        self.test_samples: int = test_samples
        self.batch_size: int = batch_size

        # define or load a model to be trained
        self.init_model()

        # get the shapes of the model weights
        self.model_shapes: List[Tuple[int, ...]] = self.get_tensorflow_shapes()

        # define or load datasets to be trained on
        self.init_datasets()
        super(Participant, self).__init__()

    def deserialize_training_input(self, data: bytes) -> TrainingInput:
        if not data:
            return TrainingInput(None)
        return TrainingInput.frombytes(data)

    def train_round(self, training_input: TrainingInput) -> TrainingResult:
        # load the weights of the global model into the local model
        self.set_tensorflow_weights(
            weights=training_input.weights, shapes=self.model_shapes
        )

        # FIXME: the epoch should come from the aggregator but I don't
        # understand what it is exactly. According to Jan it's only
        # used for metrics so I think it's ok to hardcode this to 10.
        for _ in range(0, 10):
            self.model.fit(x=self.trainset, verbose=2, shuffle=False)

        # return the updated model weights and the number of training samples
        return TrainingResult(self.get_tensorflow_weights(), self.train_samples)

    def init_weights(self) -> np.ndarray:
        self.init_model()
        return TrainingResult(self.get_tensorflow_weights(), 0)

    def init_model(self) -> None:
        """Define a simple dense neural network."""
        input_layer: Tensor = Input(shape=(self.features,), dtype="float32")
        hidden_layer: Tensor = Dense(
            units=self.units,
            activation="relu",
            use_bias=True,
            kernel_initializer="glorot_uniform",
            bias_initializer="zeros",
        )(inputs=input_layer)
        output_layer: Tensor = Dense(
            units=self.categories,
            activation="softmax",
            use_bias=True,
            kernel_initializer="glorot_uniform",
            bias_initializer="zeros",
        )(inputs=hidden_layer)
        self.model: Model = Model(inputs=[input_layer], outputs=[output_layer])
        self.model.compile(
            optimizer="Adam",
            loss="categorical_crossentropy",
            metrics=["categorical_accuracy"],
        )

    def init_datasets(self) -> None:
        """Define dummy datasets."""

        self.trainset: Dataset = Dataset.from_tensor_slices(
            tensors=(
                np.ones(shape=(self.train_samples, self.features), dtype=np.float32),
                np.tile(
                    np.eye(self.categories, dtype=np.float32),
                    reps=(int(np.ceil(self.train_samples / self.categories)), 1),
                )[0 : self.train_samples, :],
            )
        ).shuffle(buffer_size=1024).batch(batch_size=self.batch_size)
        self.valset: Dataset = Dataset.from_tensor_slices(
            tensors=(
                np.ones(shape=(self.val_samples, self.features), dtype=np.float32),
                np.tile(
                    np.eye(self.categories, dtype=np.float32),
                    reps=(int(np.ceil(self.val_samples / self.categories)), 1),
                )[0 : self.val_samples, :],
            )
        ).batch(batch_size=self.batch_size)
        self.testset: Dataset = Dataset.from_tensor_slices(
            tensors=(
                np.ones(shape=(self.test_samples, self.features), dtype=np.float32),
                np.tile(
                    np.eye(self.categories, dtype=np.float32),
                    reps=(int(np.ceil(self.test_samples / self.categories)), 1),
                )[0 : self.test_samples, :],
            )
        ).batch(batch_size=self.batch_size)

    def get_tensorflow_shapes(self) -> List[Tuple[int, ...]]:
        return [weight.shape for weight in self.model.get_weights()]

    def get_tensorflow_weights(self) -> ndarray:
        return np.concatenate(self.model.get_weights(), axis=None)

    def set_tensorflow_weights(
        self, weights: ndarray, shapes: List[Tuple[int, ...]]
    ) -> None:
        # expand the flat weights
        indices: ndarray = np.cumsum([np.prod(shape) for shape in shapes])
        tensorflow_weights: List[ndarray] = np.split(
            weights, indices_or_sections=indices
        )
        tensorflow_weights = [
            np.reshape(weight, newshape=shape)
            for weight, shape in zip(tensorflow_weights, shapes)
        ]

        # apply the weights to the tensorflow model
        self.model.set_weights(tensorflow_weights)


def main() -> None:
    """Entry point to start a participant."""

    # 50M
    # participant = Participant(
    #     features=600,
    #     units=20000,
    #     categories=25,
    #     train_samples=20800,
    #     val_samples=2600,
    #     test_samples=2600,
    #     batch_size=64,
    # )
    participant = Participant(
        features=120,
        units=2000,
        categories=4,
        train_samples=2000,
        val_samples=250,
        test_samples=250,
        batch_size=16,
    )
    run_participant("http://localhost:8081", participant)


if __name__ == "__main__":
    main()
