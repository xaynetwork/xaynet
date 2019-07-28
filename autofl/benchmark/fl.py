from typing import List, Tuple

import numpy as np
from absl import app, logging

from autofl.datasets import (
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
)
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.net import orig_cnn_compiled

from . import report

EPOCHS = 40


def benchmark_ul_fl_FashionMNIST_10p_0():
    logging.info("Starting Fashion-MNIST-10p-0 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_600.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test)


def benchmark_ul_fl_FashionMNIST_10p_1000():
    logging.info("Starting Fashion-MNIST-10p-1000 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_500_1k_bias.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test)


def benchmark_ul_fl_FashionMNIST_10p_5400():
    logging.info("Starting Fashion-MNIST-10p-5400 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_single_class.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test)


def run_unitary_versus_federated(xy_splits, xy_val, xy_test):
    # TODO initialize both (UL|FL) models the same way

    # Train CNN on a single partition ("unitary learning")
    # TODO train n models on all partitions
    partition_id = 0
    logging.info("> Train model on partition {}".format(partition_id))
    ul_results = run_uni(xy_splits[partition_id], xy_val, xy_test, EPOCHS)

    # Train CNN using federated learning on all partitions
    logging.info("> Train federated model on all partitions")
    fl_results = run_fed(xy_splits, xy_val, xy_test, EPOCHS)

    # Output results
    history_ul, loss_ul, acc_ul = ul_results
    history_fl, loss_fl, acc_fl = fl_results

    report.plot_accuracies(history_ul, history_fl)
    logging.info("UL test set loss: {}, accuracy: {}".format(loss_ul, acc_ul))
    logging.info("FL test set loss: {}, accuracy: {}".format(loss_fl, acc_fl))


def run_uni(
    xy_train: Tuple[np.ndarray, np.ndarray],
    xy_val: Tuple[np.ndarray, np.ndarray],
    xy_test: Tuple[np.ndarray, np.ndarray],
    epochs: int,
):
    # Initialize model and participant
    model = orig_cnn_compiled()
    participant = Participant(model, xy_train=xy_train, xy_val=xy_val)
    # Train model
    history = participant.train(epochs)
    # Evaluate final performance
    loss, accuracy = participant.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


# pylint: disable-msg=too-many-locals
def run_fed(
    xy_train_partitions: List[Tuple[np.ndarray, np.ndarray]],
    xy_val: Tuple[np.ndarray, np.ndarray],
    xy_test: Tuple[np.ndarray, np.ndarray],
    rounds: int,
):
    C = 3  # FIXME refactor: use fraction
    # Init participants
    participants = []
    for xy_train in xy_train_partitions:
        model = orig_cnn_compiled()
        participant = Participant(model, xy_train=xy_train, xy_val=xy_val)
        participants.append(participant)
    num_participants = len(participants)
    # Init coordinator
    model = orig_cnn_compiled()
    controller = RandomController(num_participants, C)
    coordinator = Coordinator(controller, model, participants)
    # Train model
    history = coordinator.fit(num_rounds=rounds)
    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


def main(_):
    benchmark_ul_fl_FashionMNIST_10p_0()
    # benchmark_ul_fl_FashionMNIST_10p_1000()
    # benchmark_ul_fl_FashionMNIST_10p_5400()


if __name__ == "__main__":
    app.run(main=main)
