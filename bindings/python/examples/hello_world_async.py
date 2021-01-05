"""A basic `AsyncParticipant` example"""

import logging
import time

import xaynet_sdk

LOG = logging.getLogger(__name__)


def training():
    LOG.info("training")
    time.sleep(10.0)
    LOG.info("training done")


def main() -> None:
    logging.basicConfig(
        format="%(asctime)s.%(msecs)03d %(levelname)8s %(message)s",
        level=logging.DEBUG,
        datefmt="%b %d %H:%M:%S",
    )

    (participant, global_model_notifier) = xaynet_sdk.spawn_async_participant(
        "http://127.0.0.1:8081"
    )

    try:
        while global_model_notifier.wait():
            LOG.info("a new global model")
            participant.get_global_model()
            training()
            participant.set_local_model([0.1, 0.2, 0.345, 0.3])

    except KeyboardInterrupt:
        participant.stop()


if __name__ == "__main__":
    main()
