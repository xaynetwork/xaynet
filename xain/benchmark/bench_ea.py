from absl import app, logging

from xain.benchmark.net import orig_cnn_compiled
from xain.datasets import load_splits
from xain.fl.coordinator.aggregate import EvoAgg
from xain.fl.coordinator.evaluator import Evaluator
from xain.helpers import storage

from .aggregation import task_accuracies
from .exec import run

DEFAULT_R = 50
DEFAULT_E = 1  # Number of training epochs in each round
DEFAULT_C = 0.3  # Fraction of participants used in each round of training
DEFAULT_B = 32  # Batch size used by participants


def benchmark_evolutionary_avg():
    fn_name = benchmark_evolutionary_avg.__name__
    logging.info("Starting {}".format(fn_name))

    # Load dataset
    xy_parts, xy_val, xy_test = load_splits("fashion-mnist-100p-noniid-03cpp")

    # Run Federated Learning with evolutionary aggregation
    evaluator = Evaluator(orig_cnn_compiled(), xy_val)  # FIXME refactor
    aggregator = EvoAgg(evaluator)
    hist_a, _, loss_a, acc_a = run.federated_training(
        "blog_cnn",
        xy_parts,
        xy_val,
        xy_test,
        R=DEFAULT_R,
        E=DEFAULT_E,
        C=DEFAULT_C,
        B=DEFAULT_B,
        aggregator=aggregator,
    )

    # Run Federated Learning with weighted average aggregation
    hist_b, _, loss_b, acc_b = run.federated_training(
        "blog_cnn",
        xy_parts,
        xy_val,
        xy_test,
        R=DEFAULT_R,
        E=DEFAULT_E,
        C=DEFAULT_C,
        B=DEFAULT_B,
    )

    # Output results
    task_accuracies.plot(
        [("EA", hist_a["val_acc"], None), ("WA", hist_b["val_acc"], None)],
        fname="EA-WA-plot.png",
    )

    # Write results JSON
    results = {}
    results["loss_a"] = float(loss_a)
    results["acc_a"] = float(acc_a)
    results["loss_b"] = float(loss_b)
    results["acc_b"] = float(acc_b)
    # TODO add histories
    storage.write_json(results, fname="EA-WA-results.json")


def benchmark_evolutionary_avg_with_noise():
    fn_name = benchmark_evolutionary_avg.__name__
    logging.info("Starting {}".format(fn_name))
    raise NotImplementedError()


def main(_):
    benchmark_evolutionary_avg()


if __name__ == "__main__":
    app.run(main=main)
