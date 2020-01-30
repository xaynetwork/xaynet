"""This module is the entrypoint to start a new coordinator instance.

"""
import sys

from xain_fl.config import Config, InvalidConfig, get_cmd_parameters
from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.store import Store
from xain_fl.logger import StructLogger, get_logger
from xain_fl.serve import serve

logger: StructLogger = get_logger(__name__)


def main():
    """Start a coordinator instance
    """

    args = get_cmd_parameters()
    try:
        config = Config.load(args.config)
    except InvalidConfig as err:
        logger.error("Invalid config", error=str(err))
        sys.exit(1)

    coordinator = Coordinator(
        num_rounds=config.ai.rounds,
        epochs=config.ai.epochs,
        minimum_participants_in_round=config.ai.min_participants,
        fraction_of_participants=config.ai.fraction_participants,
    )

    store = Store(config.storage)

    serve(coordinator=coordinator, store=store, server_config=config.server)


main()
