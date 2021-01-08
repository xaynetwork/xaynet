"""An `AsyncParticipant` that only downloads the latest global model"""

import json
import logging

import xaynet_sdk

LOG = logging.getLogger(__name__)


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
            global_model = participant.get_global_model()
            if global_model is not None:
                with open("global_model.bin", "w") as filehandle:
                    filehandle.write(json.dumps(global_model))

    except KeyboardInterrupt:
        participant.stop()


if __name__ == "__main__":
    main()
