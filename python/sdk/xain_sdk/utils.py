import logging


def configure_logging(level=logging.INFO, log_http_requests=False):
    if not log_http_requests:
        http_logger = logging.getLogger("xain-sdk.http")
        http_logger.setLevel(logging.WARNING)
    logging.getLogger("requests").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)

    # Disable IPython's autocompletion logger
    logging.getLogger("parso.python.diff").setLevel(logging.WARNING)
    logging.getLogger("parso.cache").setLevel(logging.WARNING)
