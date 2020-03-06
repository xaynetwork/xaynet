import pickle
from typing import List, Tuple, TypeVar

# pylint: disable=import-error
import numpy as np
from numpy import ndarray
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
    def __init__(self) -> None:
        # 3040000 Bytes = 3.04MB
        self.dummy_weights = np.array([1] * 380000)
        super(Participant, self).__init__()

    def deserialize_training_input(self, data: bytes) -> TrainingInput:
        if not data:
            return TrainingInput(None)
        return TrainingInput.frombytes(data)

    def train_round(self, training_input: TrainingInput) -> TrainingResult:
        # return the updated model weights and the number of training samples
        return TrainingResult(self.dummy_weights, 0)

    def init_weights(self) -> np.ndarray:
        return TrainingResult(self.dummy_weights, 0)

def main() -> None:
    """Entry point to start a participant."""
    participant = Participant()
    run_participant("http://localhost:8081", participant)


if __name__ == "__main__":
    main()
