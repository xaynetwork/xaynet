import os
from pathlib import Path

module_dir = os.path.dirname(__file__)  # directory in which this module resides
root_dir = os.path.abspath(os.path.join(module_dir, "../../"))  # project root dir


def root() -> Path:
    return Path(root_dir)
