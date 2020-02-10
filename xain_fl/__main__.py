"""This module is the entrypoint to start a new coordinator instance.

"""
import sys

from structlog import get_logger

from xain_fl.config import Config, InvalidConfig, get_cmd_parameters
from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.metrics_store import (
    AbstractMetricsStore,
    MetricsStore,
    NullObjectMetricsStore,
)
from xain_fl.coordinator.store import (
    NullObjectLocalWeightsReader,
    S3GlobalWeightsWriter,
)
from xain_fl.logger import StructLogger, configure_structlog
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

    configure_structlog(config.logging)

    metrics_store: AbstractMetricsStore = NullObjectMetricsStore()
    if config.metrics.enable:
        metrics_store = MetricsStore(config.metrics)

    coordinator = Coordinator(
        global_weights_writer=S3GlobalWeightsWriter(config.storage),
        local_weights_reader=NullObjectLocalWeightsReader(),
        num_rounds=config.ai.rounds,
        epochs=config.ai.epochs,
        minimum_participants_in_round=config.ai.min_participants,
        fraction_of_participants=config.ai.fraction_participants,
        metrics_store=metrics_store,
    )

    serve(coordinator=coordinator, server_config=config.server)


if __name__ == "__main__":
    main()
