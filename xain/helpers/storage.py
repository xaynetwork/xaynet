import glob
import json
import os
from typing import Dict

from absl import flags

FLAGS = flags.FLAGS


def listdir_recursive(dname: str, relpath=True):
    """Lists all files found in {dname} with relative path

    Args:
        dname (str): Absolute path to directory

    Returns:
        List[str]: List of all files with relative path to dname
    """
    files = [
        fpath
        for fpath in glob.glob(f"{dname}/**", recursive=True)
        if os.path.isfile(fpath)
    ]

    if relpath:
        return [os.path.relpath(fpath, dname) for fpath in files]

    return files


def get_abspath(fname: str, dname: str = None) -> str:

    if os.path.isabs(fname):
        return fname

    if dname is None:
        raise Exception("For relative fname dname is required")

    return os.path.join(dname, fname)


def write_json(results: Dict, fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def read_json(fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "r") as outfile:
        return json.loads(outfile.read())
