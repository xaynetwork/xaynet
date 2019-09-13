import atexit
import time

from absl import app, flags

from xain.datasets import load_splits
from xain.helpers import storage
from xain.ops import results

from . import run

FLAGS = flags.FLAGS


def after_main(group_name: str, task_name: str):
    """Will run after main exists (successfully or otherwise)"""
    # Push results once task has finished
    results.push(group_name=group_name, task_name=task_name)


def main(_):
    # Set exit callback
    if FLAGS.push_results:
        atexit.register(
            after_main, group_name=FLAGS.group_name, task_name=FLAGS.task_name
        )

    # Load data
    xy_train_partitions, xy_val, xy_test = load_splits(FLAGS.dataset)

    # Execute training
    start = time.time()
    partition_id = FLAGS.partition_id
    hist_metrics = None  # For unitary training
    if partition_id is not None:  # Use only a single partition if required (unitary)
        hist, loss, acc = run.unitary_training(
            model_name=FLAGS.model,
            xy_train=xy_train_partitions[partition_id],
            xy_val=xy_val,
            xy_test=xy_test,
            E=FLAGS.E,
            B=FLAGS.B,
        )
    else:
        hist, _, hist_metrics, loss, acc = run.federated_training(
            model_name=FLAGS.model,
            xy_train_partitions=xy_train_partitions,
            xy_val=xy_val,
            xy_test=xy_test,
            R=FLAGS.R,
            E=FLAGS.E,
            C=FLAGS.C,
            B=FLAGS.B,
        )
    end = time.time()

    # Write results
    res = {
        "group_name": FLAGS.group_name,
        "task_name": FLAGS.task_name,
        "task_label": FLAGS.task_label,
        "dataset": FLAGS.dataset,
        "model": FLAGS.model,
        "R": FLAGS.R,
        "E": FLAGS.E,
        "C": FLAGS.C,
        "B": FLAGS.B,
        "partition_id": partition_id,
        "start": start,
        "end": end,
        "duration": end - start,
        "loss": float(loss),
        "acc": float(acc),
        "hist": hist,
        "hist_metrics": hist_metrics,
    }
    storage.write_json(res, fname="results.json")


if __name__ == "__main__":
    flags.mark_flag_as_required("group_name")
    flags.mark_flag_as_required("task_name")
    flags.mark_flag_as_required("model")
    flags.mark_flag_as_required("dataset")
    flags.mark_flag_as_required("R")
    flags.mark_flag_as_required("E")
    flags.mark_flag_as_required("C")
    flags.mark_flag_as_required("B")
    # Note: Flag partition_id is not required (i.e. optional)
    app.run(main=main)
