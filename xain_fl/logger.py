"""XAIN FL structured logger"""
import logging
import os
import threading
from typing import Any

import structlog

from xain_fl.config import LoggingConfig

StructLogger = (
    structlog._config.BoundLoggerLazyProxy  # pylint: disable=protected-access
)


def configure_aimetrics_logger() -> None:
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


def add_pid_thread(_, __, event_dict) -> dict:
    """Add the pid and the name of the thread to the event dict.

     Args:
        event_dict: The event dict of structlog.

    Returns:
        The updated event dict.
    """
    pid = os.getpid()
    thread = threading.current_thread().getName()
    event_dict["pid_thread"] = f"{pid}-{thread}"
    return event_dict


def configure_structlog(config: LoggingConfig) -> None:
    """Configure structlog.

    Args:
        config: The logging config.
    """

    configure_aimetrics_logger()

    if config.console:
        shared_processors = [
            structlog.stdlib.add_logger_name,
            structlog.stdlib.add_log_level,
            structlog.stdlib.PositionalArgumentsFormatter(),
            structlog.processors.TimeStamper(fmt="%Y-%m-%d %H:%M.%S"),
            add_pid_thread,
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
        ]

        structlog.configure(
            processors=shared_processors
            + [structlog.stdlib.ProcessorFormatter.wrap_for_formatter,],
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.stdlib.BoundLogger,
            cache_logger_on_first_use=True,
        )

        formatter = structlog.stdlib.ProcessorFormatter(
            processor=structlog.dev.ConsoleRenderer(),
            foreign_pre_chain=shared_processors,
        )
    else:
        shared_processors = [
            structlog.stdlib.add_log_level,
        ]
        structlog.configure(
            processors=shared_processors
            + [structlog.stdlib.ProcessorFormatter.wrap_for_formatter,],
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.stdlib.BoundLogger,
            cache_logger_on_first_use=True,
        )

        formatter = structlog.stdlib.ProcessorFormatter(
            processor=structlog.processors.JSONRenderer(indent=2, sort_keys=True),
            foreign_pre_chain=shared_processors,
        )

    if not config.third_party:
        # disable third party logger
        for pkg_logger in logging.Logger.manager.loggerDict:  # type: ignore
            logging.getLogger(pkg_logger).propagate = False

    handler = logging.StreamHandler()
    handler.setFormatter(formatter)
    root_logger = logging.getLogger()
    root_logger.addHandler(handler)
    root_logger.setLevel(config.level.upper())
