import os

from .config import get_config

# Prepare dataset directory by creating it
# in case it does not exist
local_dataset_dir = get_config("local_dataset_dir")

if "~" in local_dataset_dir:
    local_dataset_dir = os.path.expanduser(local_dataset_dir)

local_dataset_dir = os.path.abspath(local_dataset_dir)

if not os.path.isdir(local_dataset_dir):
    os.makedirs(local_dataset_dir)
