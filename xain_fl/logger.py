"""XAIN FL structured logger"""

import logging
from typing import Any

import structlog

StructLogger = structlog._config.BoundLoggerLazyProxy  # pylint: disable=protected-access


def get_logger(
    name: str, level: int = logging.INFO
) -> structlog._config.BoundLoggerLazyProxy:  # pylint: disable=protected-access
    """Wrap python logger with default configuration of structlog.
    Args:
        name (str): Identification name. For module name pass name=__name__.
        level (int): Threshold for this logger. Defaults to logging.INFO.
    Returns:
        Wrapped python logger with default configuration of structlog.
    """
    AIMETRICS = 25  # pylint: disable=invalid-name
    structlog.stdlib.AIMETRICS = AIMETRICS
    structlog.stdlib._NAME_TO_LEVEL["aimetrics"] = AIMETRICS  # pylint: disable=protected-access
    structlog.stdlib._LEVEL_TO_NAME[AIMETRICS] = "aimetrics"  # pylint: disable=protected-access
    logging.addLevelName(AIMETRICS, "aimetrics")

    def aimetrics(self, msg: str, *args: Any, **kw: Any) -> Any:
        return self.log(AIMETRICS, msg, *args, **kw)

    structlog.stdlib._FixedFindCallerLogger.aimetrics = (  # pylint: disable=protected-access
        aimetrics
    )
    structlog.stdlib.BoundLogger.aimetrics = aimetrics
    structlog.configure(
        processors=[
            structlog.stdlib.add_log_level,
            structlog.stdlib.ProcessorFormatter.wrap_for_formatter,
        ],
        logger_factory=structlog.stdlib.LoggerFactory(),
        wrapper_class=structlog.stdlib.BoundLogger,
        cache_logger_on_first_use=True,
    )
    formatter = structlog.stdlib.ProcessorFormatter(
        processor=structlog.processors.JSONRenderer(indent=2, sort_keys=True)
    )
    handler = logging.StreamHandler()
    handler.setFormatter(formatter)
    logger = logging.getLogger(name)
    logger.addHandler(handler)
    logger.setLevel(level)

    return structlog.wrap_logger(logger)
