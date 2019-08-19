import time
from typing import List, Optional, Tuple

from absl import flags, logging

from autofl.datasets import load_splits

from . import report, run

FLAGS = flags.FLAGS

FLH_C = 0.1  # Fraction of participants used in each round of training
FLH_E = 4  # Number of training epochs in each round
FLH_B = 32  # Batch size used by participants

ROUNDS = 50


"""
In this config the key in the dictionary will be the name of the benchmark
"""
benchmarks = {
    "fashion_mnist_100p_IID_balanced": {
        "dataset_name": "fashion_mnist_100p_IID_balanced",
        "C": 0.1,
    },
    "fashion_mnist_100p_01cpp": {"dataset_name": "fashion_mnist_100p_01cpp", "C": 0.1},
    "fashion_mnist_100p_02cpp": {"dataset_name": "fashion_mnist_100p_02cpp", "C": 0.1},
    "fashion_mnist_100p_03cpp": {"dataset_name": "fashion_mnist_100p_03cpp", "C": 0.1},
    "fashion_mnist_100p_04cpp": {"dataset_name": "fashion_mnist_100p_04cpp", "C": 0.1},
    "fashion_mnist_100p_05cpp": {"dataset_name": "fashion_mnist_100p_05cpp", "C": 0.1},
    "fashion_mnist_100p_06cpp": {"dataset_name": "fashion_mnist_100p_06cpp", "C": 0.1},
    "fashion_mnist_100p_07cpp": {"dataset_name": "fashion_mnist_100p_07cpp", "C": 0.1},
    "fashion_mnist_100p_08cpp": {"dataset_name": "fashion_mnist_100p_08cpp", "C": 0.1},
    "fashion_mnist_100p_09cpp": {"dataset_name": "fashion_mnist_100p_09cpp", "C": 0.1},
    "fashion_mnist_100p_10cpp": {"dataset_name": "fashion_mnist_100p_10cpp", "C": 0.1},
}


def _run_unitary_versus_federated(benchmark_name: str, dataset_name: str, C: float):
    """
    :param C: Fraction of participants used in each round of training
    """
    logging.info(f"Starting {benchmark_name}")
    xy_splits, xy_val, xy_test = load_splits(dataset_name)

    start = time.time()

    # Train CNN on a single partition ("unitary learning")
    # TODO train n models on all partitions
    partition_id = 0
    logging.info(f"Run unitary training using partition {partition_id}")
    ul_hist, ul_loss, ul_acc = run.unitary_training(
        xy_splits[partition_id],
        xy_val,
        xy_test,
        epochs=ROUNDS * FLH_E,
        batch_size=FLH_B,
    )

    # Train CNN using federated learning on all partitions
    logging.info("Run federated learning using all partitions")
    fl_hist, _, fl_loss, fl_acc = run.federated_training(
        xy_splits, xy_val, xy_test, ROUNDS, C=C, E=FLH_E, B=FLH_B
    )

    end = time.time()

    # Write results JSON
    results = {
        "name": benchmark_name,
        "start": start,
        "end": end,
        "duration": end - start,
        "FLH_C": C,
        "FLH_E": FLH_E,
        "FLH_B": FLH_B,
        "ROUNDS": ROUNDS,
        "unitary_learning": {
            "loss": float(ul_loss),
            "acc": float(ul_acc),
            "hist": ul_hist,
        },
        "federated_learning": {
            "loss": float(fl_loss),
            "acc": float(fl_acc),
            "hist": fl_hist,
        },
    }
    report.write_json(results, fname="results.json")

    # Plot results
    # TODO include aggregated participant histories in plot
    plot_data: List[Tuple[str, List[float], Optional[List[int]]]] = [
        (
            "Unitary Learning",
            ul_hist["val_acc"],
            [i for i in range(1, len(ul_hist["val_acc"]) + 1, 1)],
        ),
        (
            "Federated Learning",
            fl_hist["val_acc"],
            [i for i in range(FLH_E, len(fl_hist["val_acc"]) * FLH_E + 1, FLH_E)],
        ),
    ]
    # FIXME use different filenames for different datasets
    report.plot_accuracies(plot_data, fname="plot.png")


def main():
    benchmark_name = FLAGS.benchmark_name
    kwargs = benchmarks[benchmark_name]
    _run_unitary_versus_federated(benchmark_name=benchmark_name, **kwargs)
