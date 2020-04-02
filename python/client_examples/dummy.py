import argparse
from io import BytesIO
import logging
import threading

# pylint: disable=import-error
import numpy as np
from xain_sdk import ParticipantABC, configure_logging, run_participant

LOG = logging.getLogger(__name__)


class Participant(ParticipantABC):
    def __init__(self, model: bytes) -> None:
        self.model = model
        super(Participant, self).__init__()

    def deserialize_training_input(self, data: bytes) -> bytes:
        return data

    def serialize_training_result(self, _result: bytes) -> bytes:
        return self.model

    def train_round(self, training_input: bytes) -> bytes:
        return self.model


def participant_worker(participant, url, heartbeat_period, exit_event):
    try:
        run_participant(participant, url, heartbeat_period=heartbeat_period)
    except KeyboardInterrupt:
        exit_event.set()
    # pylint: disable=bare-except
    except:
        LOG.exception("participant exited with an error")
        exit_event.set()
    else:
        exit_event.set()


ARRAY_LENGTHS_BY_SIZE = {
    "100B": 0,  # 159 B
    "1kB": 218,  # 1042 B
    "1MB": 264_000,  # 1_056_165 B
    "5MB": 1_310_000,  # 5_240_165 B
    "10MB": 13_108_000,  # 52_432_165 B
    "50MB": 26_215_000,  # 104_860_165 B
}


def generate_training_result(size: str) -> bytes:
    """Generate the data sent to the aggregator after training"""
    # Create the array
    array_length = ARRAY_LENGTHS_BY_SIZE[size]
    weights = np.ones((array_length,), dtype=np.float32)
    # Serialize the array
    writer = BytesIO()
    writer.write(int(0).to_bytes(4, byteorder="big"))
    np.save(writer, weights, allow_pickle=False)
    return writer.getvalue()


def human_readable_size(size: int) -> str:
    if size < 1024:
        return f"{size}B"
    if size < 1024 * 1024:
        kb_size = round(size / 1024, 2)
        return f"{kb_size}kB"
    mb_size = round(size / (1024 * 1024), 2)
    return f"{mb_size}MB"


def main(
    size: str, number_of_participants: int, coordinator_url: str, heartbeat_period: int,
) -> None:
    """Entry point to start a participant."""
    training_result_data = generate_training_result(size)
    LOG.info("training data size: %s", human_readable_size(len(training_result_data)))

    if number_of_participants < 2:
        participant = Participant(training_result_data)
        run_participant(participant, coordinator_url, heartbeat_period=heartbeat_period)
        return

    exit_event = threading.Event()
    threads = []
    for _ in range(0, number_of_participants):
        participant = Participant(training_result_data)
        thread = threading.Thread(
            target=participant_worker,
            args=(participant, coordinator_url, heartbeat_period, exit_event),
        )
        thread.daemon = True
        thread.start()
        threads.append(thread)

    def join_threads() -> None:
        for thread in threads:
            thread.join()
        LOG.info("all participants finished")
        exit_event.set()

    monitor = threading.Thread(target=join_threads)
    monitor.daemon = True
    monitor.start()
    exit_event.wait()


if __name__ == "__main__":
    # pylint: disable=invalid-name
    logging.basicConfig(
        format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
        level=logging.DEBUG,
        datefmt="%Y-%m-%d %H:%M:%S",
    )

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
        choices=["100B", "1kB", "1MB", "5MB", "10MB", "50MB"],
        type=str,
        default="1kB",
        help="Size of the model to send to the aggregator",
    )
    parser.add_argument(
        "--heartbeat-period", type=float, default=1, help="Heartbeat period in seconds",
    )
    parser.add_argument(
        "--verbose", action="store_true", help="Log the HTTP requests",
    )
    args = parser.parse_args()

    if args.verbose:
        configure_logging(log_http_requests=True)
    else:
        configure_logging(log_http_requests=False)

    main(
        args.model_size,
        args.number_of_participants,
        args.coordinator_url,
        args.heartbeat_period,
    )
