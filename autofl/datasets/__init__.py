import os

from absl import flags

FLAGS = flags.FLAGS

flags.DEFINE_string(
    "local_datasets_dir",
    os.path.expanduser("~/.autofl/datasets"),
    "Local directory to store datasets in. Usually ~/.autofl/datasets",
)
flags.DEFINE_string(
    "datasets_repository",
    "http://datasets.xain.io/autofl",
    "Remote datasets repository.",
)
flags.DEFINE_boolean(
    "fetch_datasets",
    False,
    "Indicates if remote datasets should be fetched if required.",
)
