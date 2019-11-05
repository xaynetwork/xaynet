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


def create_output_subdir(dname: str) -> str:
    if os.path.isabs(dname):
        raise Exception("Please provide a relative directory name")

    dname = os.path.join(FLAGS.output_dir, dname)

    os.makedirs(dname, exist_ok=True)

    return dname


def fname_with_default_dir(fname: str, dname: str = None) -> str:
    """Returns fname if its a absolute path otherwise joins it with dname"""
    if os.path.isabs(fname):
        return fname

    if dname is None:
        raise Exception("For relative fname dname is required")

    return os.path.join(dname, fname)


def write_json(results: Dict, fname: str):
    fname = fname_with_default_dir(fname, FLAGS.output_dir)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def read_json(fname: str):
    fname = fname_with_default_dir(fname, FLAGS.output_dir)
    with open(fname, "r") as outfile:
        return json.loads(outfile.read())
