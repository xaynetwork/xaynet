import os

from absl import flags

from .dataset import load_splits

FLAGS = flags.FLAGS

flags.DEFINE_string(
    "local_datasets_dir",
    os.path.expanduser("~/.xain/datasets"),
    "Local directory to store datasets in. Usually ~/.xain/datasets",
)
flags.DEFINE_string(
    "datasets_repository", "http://datasets.xain.io", "Remote datasets repository."
)
flags.DEFINE_boolean(
    "fetch_datasets",
    True,
    "Indicates if remote datasets should be fetched if required.",
)
