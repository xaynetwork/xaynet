import logging
import threading
import argparse
import pickle

# pylint: disable=import-error
import numpy as np
from xain_sdk import (
    ParticipantABC,
    TrainingInputABC,
    TrainingResultABC,
    run_participant,
)

LOG = logging.getLogger(__name__)


class TrainingInput(TrainingInputABC):
    def is_initialization_round(self) -> bool:
        return False


class TrainingResult(TrainingResultABC):
    def __init__(self, data: bytes):
        self.data = data

    def tobytes(self) -> bytes:
        return self.data


class Participant(ParticipantABC):
    def __init__(self, model: bytes) -> None:
        self.training_input = TrainingInput()
        self.training_result = TrainingResult(model)
        super(Participant, self).__init__()

    def deserialize_training_input(self, data: bytes) -> TrainingInput:
        return self.training_input

    def train_round(self, training_input: TrainingInput) -> TrainingResult:
        return self.training_result

    def init_weights(self) -> TrainingResult:
        return self.training_result


def participant_worker(participant, url, exit_event):
    try:
        run_participant(participant, url)
    except KeyboardInterrupt:
        exit_event.set()
        return
    # pylint: disable=bare-except
    except:
        LOG.exception("participant exited with an error")
        exit_event.set()
        return


def main(size: int, number_of_participants: int, coordinator_url: str) -> None:
    """Entry point to start a participant."""
    weights = np.array([1] * size)
    training_result_data = int(0).to_bytes(4, byteorder="big") + pickle.dumps(weights)

    if number_of_participants < 2:
        participant = Participant(training_result_data)
        run_participant(participant, coordinator_url)
    else:
        exit_event = threading.Event()
        threads = []
        for _ in range(0, number_of_participants):
            participant = Participant(training_result_data)
            thread = threading.Thread(
                target=participant_worker,
                args=(participant, coordinator_url, exit_event),
            )
            thread.daemon = True
            thread.start()
            threads.append(thread)
        exit_event.wait()


if __name__ == "__main__":
    # pylint: disable=invalid-name
    logging.basicConfig(level=logging.DEBUG)

    parser = argparse.ArgumentParser(description="Run dummy participants")
    parser.add_argument(
        "--number-of-participants",
        type=int,
        default=1,
        help="number of participants to start",
    )
    parser.add_argument(
        "--coordinator-url", type=str, required=True, help="URL of the coordinator",
    )
    parser.add_argument(
        "--model-size",
        type=int,
        # The default value corresponds roughly to a payload of 1MB
        default=125_000,
        help="Number of weights to use",
    )
    args = parser.parse_args()
    main(args.model_size, args.number_of_participants, args.coordinator_url)
