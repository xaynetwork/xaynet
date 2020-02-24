"""The entrypoint to start a new coordinator instance."""

import sys

from structlog import get_logger

from xain_fl.config import Config, InvalidConfigError, get_cmd_parameters
from xain_fl.coordinator.coordinator import Coordinator, Participants
from xain_fl.coordinator.metrics_store import (
    AbstractMetricsStore,
    MetricsStore,
    NullObjectMetricsStore,
)
from xain_fl.coordinator.store import S3GlobalWeightsWriter, S3LocalWeightsReader
from xain_fl.logger import StructLogger, configure_structlog
from xain_fl.serve import serve

logger: StructLogger = get_logger(__name__)


def main() -> None:
    """Start a coordinator instance."""

    args = get_cmd_parameters()
    try:
        config = Config.load(args.config)
    except InvalidConfigError as err:
        logger.error("Invalid config", error=str(err))
        sys.exit(1)

    configure_structlog(config.logging)

    metrics_store: AbstractMetricsStore = NullObjectMetricsStore()
    if config.metrics.enable:  # type: ignore
        metrics_store = MetricsStore(config.metrics)

    coordinator = Coordinator(
        global_weights_writer=S3GlobalWeightsWriter(config.storage),
        local_weights_reader=S3LocalWeightsReader(config.storage),
        num_rounds=config.ai.rounds,  # type: ignore
        epochs=config.ai.epochs,  # type: ignore
        minimum_participants_in_round=config.ai.min_participants,  # type: ignore
        fraction_of_participants=config.ai.fraction_participants,  # type: ignore
        metrics_store=metrics_store,
        participants=Participants(
            heartbeat_time=config.server.heartbeat_time,  # type: ignore
            heartbeat_timeout=config.server.heartbeat_timeout,  # type: ignore
        ),
    )

    serve(coordinator=coordinator, server_config=config.server)


if __name__ == "__main__":
    main()
