"""
This file contains global config variables
"""
import os

config = {
    "local_datasets_dir": os.environ.get("LOCAL_DATASETS_DIR", "~/.autofl/datasets"),
    "fetch_datasets": os.environ.get("FETCH_DATASETS", "1"),
}


def get_config(config_name: str) -> str:
    """
    Takes and config from config but overrides in case their is an ENV with equal name set
    """
    return config[config_name]
