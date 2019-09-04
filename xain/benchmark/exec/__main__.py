import time

from absl import app, flags

from xain.datasets import load_splits
from xain.helpers import storage
from xain.ops import results

from . import run

FLAGS = flags.FLAGS


def main(_):
    # Load data
    xy_train_partitions, xy_val, xy_test = load_splits(FLAGS.dataset)

    # Execute training
    start = time.time()
    hist, _, loss, acc = run.federated_training(
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
        "name": FLAGS.dataset,  # FIXME remove
        "group_name": FLAGS.group_name,
        "task_name": FLAGS.task_name,
        "dataset": FLAGS.dataset,
        "model": FLAGS.model,
        "R": FLAGS.R,
        "E": FLAGS.E,
        "C": FLAGS.C,
        "B": FLAGS.B,
        "start": start,
        "end": end,
        "duration": end - start,
        "loss": float(loss),
        "acc": float(acc),
        "hist": hist,
    }
    storage.write_json(res, fname="results.json")

    # Push results once task has finished
    results.push(group_name=FLAGS.group_name, task_name=FLAGS.task_name)


if __name__ == "__main__":
    flags.mark_flag_as_required("group_name")
    flags.mark_flag_as_required("task_name")
    flags.mark_flag_as_required("model")
    flags.mark_flag_as_required("dataset")
    flags.mark_flag_as_required("R")
    flags.mark_flag_as_required("E")
    flags.mark_flag_as_required("C")
    flags.mark_flag_as_required("B")
    app.run(main=main)
