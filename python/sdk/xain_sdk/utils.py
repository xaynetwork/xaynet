import logging


def configure_logging(log_http_requests=False):
    # These loggers are extremely verbose, and redundant with
    # `xain-sdk.http` so we set them to a higher level
    logging.getLogger("requests").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    if not log_http_requests:
        http_logger = logging.getLogger("xain-sdk.http")
        http_logger.setLevel(logging.WARNING)

    # Disable IPython's autocompletion logger. This is more convenient
    # for interactive debugging, but arguably it be removed at some
    # point
    logging.getLogger("parso.python.diff").setLevel(logging.WARNING)
    logging.getLogger("parso.cache").setLevel(logging.WARNING)
