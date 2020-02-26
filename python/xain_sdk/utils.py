import logging


def configure_logging(level=logging.DEBUG):
    logging.basicConfig(level=level)
    logging.getLogger("requests").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    # Disable IPython's autocompletion logger
    logging.getLogger("parso.python.diff").setLevel(logging.WARNING)
    logging.getLogger("parso.cache").setLevel(logging.WARNING)
