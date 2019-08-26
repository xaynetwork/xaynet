from absl import app, logging

from autofl.datasets.dataset import config, load_splits

from .stats import DSStats


def main(_):
    for dataset_name in config:
        logging.info(dataset_name)
        logging.info(DSStats(name=dataset_name, ds=load_splits(dataset_name)))


app.run(main=main)
