import configparser
import os
from functools import lru_cache
from pathlib import Path

from xain_fl.helpers import project
from xain_fl.logger import get_logger

# Storage dir for bigger files like the datasets
storage_dir = Path.home().joinpath(".xain-fl")
datasets_dir_default = storage_dir.joinpath("datasets")

# Local outputs and remote results
root_dir = project.root()
config_file = root_dir.joinpath("config.cfg")
output_dir_default = root_dir.joinpath("output")
results_dir_default = root_dir.joinpath("results")

logger = get_logger(__name__, level=os.environ.get("XAIN_LOGLEVEL", "INFO"))


def init_config():
    """Creates initial config file if non exists"""
    logger.info("Initializing config in %s", config_file)

    config = configparser.ConfigParser()

    # Path config section
    config.add_section("Path")
    config.set("Path", "output_dir", str(output_dir_default))
    config.set("Path", "results_dir", str(results_dir_default))
    config.set("Path", "local_datasets_dir", str(datasets_dir_default))

    config.add_section("S3")
    config.set(
        "S3",
        "results_bucket",
        # Using an ENV variable here to we can set it on the CI as an environment variable
        os.environ.get(
            "S3_RESULTS_BUCKET",
            default="ACCESSIBLE_S3_BUCKET_FOR_RESULTS_TO_BE_UPLOADED",
        ),
    )

    # Dataset config section
    config.add_section("Dataset")
    config.set(
        "Dataset",
        "repository",
        "http://xain-datasets.s3-website.eu-central-1.amazonaws.com",
    )
    config.set(
        "Dataset", "fetch_datasets", "True"
    )  # Indicates if datasets should be fetched from remote by default

    with open(config_file, "w") as configfile:
        config.write(configfile)


def check_config_file_exists():
    return os.path.isfile(config_file)


@lru_cache()  # Using this to avoid loading file every time from disk
def load():
    config = configparser.ConfigParser()
    config.read(config_file)
    return config
