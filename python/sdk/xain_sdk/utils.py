import logging


def configure_logging(level=logging.INFO):
    logging.basicConfig(level=level, format="%(asctime)s %(levelname)-8s %(message)s")
    http_logger = logging.getLogger("xain-sdk.http")
    http_logger.setLevel(logging.WARNING)
    logging.getLogger("requests").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)

    # Disable IPython's autocompletion logger
    logging.getLogger("parso.python.diff").setLevel(logging.WARNING)
    logging.getLogger("parso.cache").setLevel(logging.WARNING)
