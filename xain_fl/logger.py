"""This module contains custom logging configuration"""
import logging
import sys

import structlog


def get_logger(name: str, level: str = "INFO") -> structlog.BoundLogger:
    """Returns an instance of the custom xain-fl logger.

    Args:
        name (:obj:`str`): The name of the logger. Typically `__name__`.
        level (int): Threshold for this logger. Can be one of `CRITICAL`,
            `ERROR`, `WARNING`, `INFO`, `DEBUG`. Defaults to `INFO`.

    Returns:
        :class:`~structlog.BoundLogger`: Configured logger.
    """

    logging.basicConfig(format="%(message)s", stream=sys.stdout, level=level)

    structlog.configure(
        processors=[
            structlog.processors.StackInfoRenderer(),
            structlog.dev.set_exc_info,
            structlog.processors.format_exc_info,
            structlog.stdlib.add_logger_name,
            structlog.stdlib.add_log_level,
            structlog.processors.TimeStamper(),
            structlog.processors.JSONRenderer(indent=None, sort_keys=True),
        ],
        wrapper_class=structlog.BoundLogger,
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=False,
    )

    logger = structlog.get_logger(name)

    return logger
