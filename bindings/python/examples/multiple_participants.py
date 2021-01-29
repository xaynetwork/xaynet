"""Spawn multiple `ParticipantABC`s in a single process"""

import json
import logging
import time
from typing import Optional

import xaynet_sdk

LOG = logging.getLogger(__name__)


class Participant(xaynet_sdk.ParticipantABC):
    def __init__(self, p_id: int, model: list) -> None:
        self.p_id = p_id
        self.model = model
        super().__init__()

    def deserialize_training_input(self, global_model: list) -> list:
        return global_model

    def train_round(self, training_input: Optional[list]) -> list:
        LOG.info("participant %s: start training", self.p_id)
        time.sleep(5.0)
        LOG.info("participant %s: training done", self.p_id)
        return self.model

    def serialize_training_result(self, training_result: list) -> list:
        return training_result

    def participate_in_update_task(self) -> bool:
        return True

    def on_new_global_model(self, global_model: Optional[list]) -> None:
        if global_model is not None:
            with open("global_model.bin", "w") as filehandle:
                filehandle.write(json.dumps(global_model))


def main() -> None:
    logging.basicConfig(
        format="%(asctime)s.%(msecs)03d %(levelname)8s %(message)s",
        level=logging.DEBUG,
        datefmt="%b %d %H:%M:%S",
    )

    participant = xaynet_sdk.spawn_participant(
        "http://127.0.0.1:8081",
        Participant,
        args=(
            1,
            [0.1, 0.2, 0.345, 0.3],
        ),
    )

    participant_2 = xaynet_sdk.spawn_participant(
        "http://127.0.0.1:8081",
        Participant,
        args=(
            2,
            [0.3, 0.4, 0.45, 0.1],
        ),
    )

    participant_3 = xaynet_sdk.spawn_participant(
        "http://127.0.0.1:8081",
        Participant,
        args=(
            3,
            [0.123, 0.1567, 0.123, 0.46],
        ),
    )

    try:
        participant.join()
        participant_2.join()
        participant_3.join()
    except KeyboardInterrupt:
        participant.stop()
        participant_2.stop()
        participant_3.stop()


if __name__ == "__main__":
    main()
