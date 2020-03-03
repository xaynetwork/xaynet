import pickle
from typing import TypeVar

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
    def frombytes(data: bytes) -> TrainingInput:
        return TrainingInput(pickle.loads(data))

    def is_initialization_round(self) -> bool:
        return self.weights.size == 0


class TrainingResult(TrainingResultABC):
    def __init__(self, weights: ndarray):
        self.weights = weights

    def tobytes(self) -> bytes:
        return pickle.dumps(self.weights)


class Participant(ParticipantABC):
    def train_round(self, training_input: TrainingInput) -> TrainingResult:
        return training_input.weights

    def init_weights(self) -> TrainingResult:
        return TrainingResult(np.ndarray([1, 2, 3, 4]))


if __name__ == "__main__":
    run_participant("http://localhost:8081", Participant())
