"""This module contains custom logging configuration"""


import logging


def get_logger(name: str, level: str = "INFO") -> logging.Logger:
    """Returns an instance of the custom xain-fl logger.

    Args:
        name (:obj:`str`): The name of the logger. Typically `__name__`.
        level (int): Threshold for this logger. Can be one of `CRITICAL`,
            `ERROR`, `WARNING`, `INFO`, `DEBUG`. Defaults to `INFO`.

    Returns:
        :class:`~logging.Logger`: Configured logger.
    """

    logging.basicConfig(
        format="[%(asctime)s] [%(levelname)s] (%(name)s) %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
        level=level,
    )
    return logging.getLogger(name)
