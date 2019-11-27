"""Provides compositions of aggregation functions identified by unique names
to be used in benchmark scenario configuration
"""
import os
from typing import Callable, Dict

from absl import flags, logging

from benchmarks.benchmark.aggregation import (
    final_task_accuracies,
    learning_rate,
    participant_hist,
    task_accuracies,
)
from benchmarks.helpers import storage

FLAGS = flags.FLAGS


def _aggregate():
    """Calls aggregation defined in a benchmark groups config.json file"""
    fname = os.path.join(FLAGS.results_dir, FLAGS.group_name, "config.json")
    config = storage.read_json(fname)

    aggregation_name = config["aggregation_name"]

    aggregations[aggregation_name]()


def _flul_aggregation():
    logging.info("flul_aggregation started")
    task_accuracies.aggregate()
    learning_rate.aggregate()
    participant_hist.participant_history()


def _cpp_aggregation():
    logging.info("cpp_aggregation started")
    task_accuracies.aggregate()
    final_task_accuracies.aggregate()
    participant_hist.participant_history()


aggregations: Dict[str, Callable] = {
    "flul-aggregation": _flul_aggregation,
    "cpp-aggregation": _cpp_aggregation,
    "vol-aggregation": _flul_aggregation,
}


def main(_):
    """Used by ~benchmarks.aggregate.main to create an aggregation of benchmark results
    identified  by the commandline flag `--group_name`. Has to be invoked through
    abseil `app.run`.
    """
    _aggregate()
