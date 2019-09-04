import time
from typing import List, Optional, Tuple

import tensorflow as tf
from absl import app, logging

from xain.helpers import storage

from .exec import report, run

B = 64


def bench_cl90(_):
    """
    This benchmark compares training on the full (original) Fashion-MNIST dataset using
    (a) the participant directly and (b) the coordinator with only one participant.
    Results should not vary apart from the expected non-determinism during training.
    """
    logging.info("Starting CL on full Fashion-MNIST")
    # Load original Fashion-MNIST
    xy_train, xy_val, xy_test = data(limit=False)
    bench_cl_ul(
        "cl90-using-participant",
        xy_train,
        xy_val,
        xy_test,
        epochs=10,
        use_coordinator=False,
    )
    bench_cl_ul(
        "cl90-using-coordinator",
        xy_train,
        xy_val,
        xy_test,
        epochs=10,
        use_coordinator=True,
    )


def bench_ul80(_):
    """
    This benchmark compares training on a subset (550 examples) of the original
    Fashion-MNIST dataset using (a) the participant directly and (b) the coordinator with
    only one participant. Results should not vary apart from the usual non-determinism
    during training.
    """
    logging.info("Starting UL on 550-example Fashion-MNIST subset")
    # Load original Fashion-MNIST subset
    xy_train, xy_val, xy_test = data(limit=True)
    bench_cl_ul(
        "ul80-using-participant",
        xy_train,
        xy_val,
        xy_test,
        epochs=100,
        use_coordinator=False,
    )
    bench_cl_ul(
        "ul80-using-coordinator",
        xy_train,
        xy_val,
        xy_test,
        epochs=100,
        use_coordinator=True,
    )


def bench_cl_ul(
    name: str, xy_train, xy_val, xy_test, epochs: int, use_coordinator: bool
):
    start = time.time()
    if use_coordinator:
        hist, _, loss, acc = run.federated_training(
            "blog_cnn", [xy_train], xy_val, xy_test, R=epochs, E=1, C=0, B=B
        )
    else:
        hist, loss, acc = run.unitary_training(
            "blog_cnn", xy_train, xy_val, xy_test, E=epochs, B=B
        )
    end = time.time()

    # Write results JSON
    results = {
        "name": name,
        "start": start,
        "end": end,
        "duration": end - start,
        "E": epochs,
        "B": B,
        "unitary_learning": {"loss": float(loss), "acc": float(acc), "hist": hist},
    }
    storage.write_json(results, fname=name + "-results.json")

    # Plot results
    plot_data: List[Tuple[str, List[float], Optional[List[int]]]] = [
        (
            "Unitary Learning",
            hist["val_acc"],
            [i for i in range(1, len(hist["val_acc"]) + 1, 1)],
        )
    ]
    report.plot_accuracies(plot_data, fname=name + "-plot.png")


def data(limit: bool):
    (x_train, y_train), (x_test, y_test) = tf.keras.datasets.fashion_mnist.load_data()
    # Split xy_val
    (x_train, x_valid) = x_train[5000:], x_train[:5000]
    (y_train, y_valid) = y_train[5000:], y_train[:5000]
    # Subset
    if limit:
        x_train = x_train[:550]
        y_train = y_train[:550]
    return (x_train, y_train), (x_valid, y_valid), (x_test, y_test)


if __name__ == "__main__":
    app.run(main=bench_ul80)
