import os

module_dir = os.path.dirname(__file__)  # directory in which this module resides
root_dir = os.path.abspath(os.path.join(module_dir, "../../"))  # project root dir


def root():
    return root_dir
