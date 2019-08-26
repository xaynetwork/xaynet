import json
import os
from typing import Dict

from absl import flags

FLAGS = flags.FLAGS


def get_abspath(fname: str, fdir: str = None) -> str:
    if os.path.isabs(fname):
        return fname

    if fdir is None:
        raise Exception("For relative fname fdir is required")

    return os.path.join(fdir, fname)


def write_json(results: Dict, fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def read_json(fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "r") as outfile:
        return json.loads(outfile.read())
