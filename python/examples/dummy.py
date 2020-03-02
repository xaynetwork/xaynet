from xain_sdk import Participant, run_participant

import numpy as np
from numpy import ndarray


class DummyParticipant(Participant):
    def train_round(self, weights: ndarray, _epochs: int, _epoch_base: int):
        return weights

    def init_weights(self):
        np.ndarray([1, 2, 3, 4])


if __name__ == "__main__":
    dummy_participant = DummyParticipant()
    run_participant("http://localhost:8081", dummy_participant)
