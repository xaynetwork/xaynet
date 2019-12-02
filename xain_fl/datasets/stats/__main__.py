import os

from absl import app

from xain_fl.datasets.dataset import config, load_splits

from .stats import DSStats

module_dir = os.path.dirname(__file__)  # directory in which this module resides
stats_dir = os.path.abspath(os.path.join(module_dir, "datasets"))  # project root dir


def main(_):
    for dataset_name in config:
        fname = os.path.join(stats_dir, f"{dataset_name}.txt")
        with open(fname, "w") as f:
            s = DSStats(name=dataset_name, ds=load_splits(dataset_name)).__repr__()

            # Don't log with xain_fl.logger as repl in DSStats expects to
            # be printed with print
            print(s)

            f.write(s)


app.run(main=main)
