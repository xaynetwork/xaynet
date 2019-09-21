import os
from typing import Callable, Dict

from absl import flags, logging

from xain.benchmark.aggregation import (
    final_task_accuracies,
    learning_rate,
    task_accuracies,
)
from xain.helpers import storage

FLAGS = flags.FLAGS


def aggregate():
    """Calls aggregation defined in group config.json"""
    fname = os.path.join(FLAGS.results_dir, FLAGS.group_name, "config.json")
    config = storage.read_json(fname)

    aggregation_name = config["aggregation_name"]

    aggregations[aggregation_name]()


def flul_aggregation():
    logging.info("flul_aggregation started")
    task_accuracies.aggregate()
    learning_rate.aggregate()


def cpp_aggregation():
    logging.info("cpp_aggregation started")
    task_accuracies.aggregate()
    final_task_accuracies.aggregate()


aggregations: Dict[str, Callable] = {
    "flul-aggregation": flul_aggregation,
    "cpp-aggregation": cpp_aggregation,
    "vol-aggregation": flul_aggregation,
}
