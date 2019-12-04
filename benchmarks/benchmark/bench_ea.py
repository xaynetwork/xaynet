"""Experimental"""
from absl import app

from benchmarks.benchmark.exec import run
from benchmarks.benchmark.net import orig_cnn_compiled
from benchmarks.helpers import storage
from xain_fl.datasets import load_splits
from xain_fl.fl.coordinator.aggregate import EvoAgg
from xain_fl.fl.coordinator.evaluator import Evaluator
from xain_fl.logger import get_logger

logger = get_logger(__name__)


DEFAULT_R = 50
DEFAULT_E = 1  # Number of training epochs in each round
DEFAULT_C = 0.3  # Fraction of participants used in each round of training
DEFAULT_B = 32  # Batch size used by participants


def benchmark_evolutionary_avg():
    fn_name = benchmark_evolutionary_avg.__name__
    logger.info("Starting {}".format(fn_name))

    # Load dataset
    xy_parts, xy_val, xy_test = load_splits("fashion-mnist-100p-noniid-03cpp")

    # Run Federated Learning with evolutionary aggregation
    evaluator = Evaluator(orig_cnn_compiled(), xy_val)  # FIXME refactor
    aggregator = EvoAgg(evaluator)
    _, _, _, _, loss_a, acc_a = run.federated_training(
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
    _, _, _, _, loss_b, acc_b = run.federated_training(
        "blog_cnn",
        xy_parts,
        xy_val,
        xy_test,
        R=DEFAULT_R,
        E=DEFAULT_E,
        C=DEFAULT_C,
        B=DEFAULT_B,
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
    logger.info("Starting {}".format(fn_name))
    raise NotImplementedError()


def main(_):
    benchmark_evolutionary_avg()


if __name__ == "__main__":
    app.run(main=main)
