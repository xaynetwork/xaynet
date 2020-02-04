"""XAIN FL structured logger"""

import logging
from typing import Any, Optional, Union

import structlog

StructLogger = (
    structlog._config.BoundLoggerLazyProxy  # pylint: disable=protected-access
)


def configure_aimetrics_logger():
    """Configure a logger named "aimetrics" with a configurable log
    level.

    """
    AIMETRICS = 25  # pylint: disable=invalid-name
    structlog.stdlib.AIMETRICS = AIMETRICS
    structlog.stdlib._NAME_TO_LEVEL[  # pylint: disable=protected-access
        "aimetrics"
    ] = AIMETRICS
    structlog.stdlib._LEVEL_TO_NAME[  # pylint: disable=protected-access
        AIMETRICS
    ] = "aimetrics"
    logging.addLevelName(AIMETRICS, "aimetrics")

    def aimetrics(self, msg: str, *args: Any, **kw: Any) -> Any:
        return self.log(AIMETRICS, msg, *args, **kw)

    structlog.stdlib._FixedFindCallerLogger.aimetrics = (  # pylint: disable=protected-access
        aimetrics
    )
    structlog.stdlib.BoundLogger.aimetrics = aimetrics


def configure_structlog():
    """Set the structlog configuration"""
    structlog.configure(
        processors=[
            structlog.stdlib.add_log_level,
            structlog.stdlib.ProcessorFormatter.wrap_for_formatter,
        ],
        logger_factory=structlog.stdlib.LoggerFactory(),
        wrapper_class=structlog.stdlib.BoundLogger,
        cache_logger_on_first_use=True,
    )


def set_log_level(level: Union[str, int]):
    """Set the log level on the root logger. Since by default, the root
    logger log level is inherited by all the loggers, this is like
    setting a default log level.

    Args:

        level: the log level, as documented in the `Python standard
            library <https://docs.python.org/3/library/logging.html#levels>`_
    """
    root_logger = logging.getLogger()
    root_logger.setLevel(level)


def initialize_logging():
    """Set up logging

    """
    configure_aimetrics_logger()
    configure_structlog()


def get_logger(
    name: str, level: Optional[int] = None
) -> structlog._config.BoundLoggerLazyProxy:  # pylint: disable=protected-access
    """Wrap python logger with default configuration of structlog.

    Args:
        name: Identification name. For module name pass ``name=__name__``.
        level: Threshold for this logger.

    Returns:
        Wrapped python logger with default configuration of structlog.
    """
    formatter = structlog.stdlib.ProcessorFormatter(
        processor=structlog.processors.JSONRenderer(indent=2, sort_keys=True)
    )
    handler = logging.StreamHandler()
    handler.setFormatter(formatter)
    logger = logging.getLogger(name)
    logger.addHandler(handler)

    if level is not None:
        logger.setLevel(level)

    return structlog.wrap_logger(logger)
