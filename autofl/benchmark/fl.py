from typing import List, Tuple

import numpy as np
from absl import app, logging

from autofl.datasets import fashion_mnist_10s_600
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.net import cnn_compiled

from . import report


# TODO initialize both (UL|FL) models the same way
def benchmark_gain_FashionMNIST():
    logging.info("Starting Fashion-MNIST Benchmark")

    EPOCHS = 40

    # Load perfectly balanced shards
    xy_splits, xy_test = fashion_mnist_10s_600.load_splits()

    # Train CNN on a single partition ("unitary learning")
    partition_id = 0
    logging.info("> Train model on partition {}".format(partition_id))
    # TODO use xy_val once available
    ul_results = run_uni(xy_splits[partition_id], xy_test, xy_test, EPOCHS)

    # Train CNN using federated learning on all partitions
    logging.info("> Train federated model on all partitions")
    # TODO use xy_val once available
    fl_results = run_fed(xy_splits, xy_test, xy_test, EPOCHS)

    # Output results
    history_ul, loss_ul, accuracy_ul = ul_results
    history_fl, loss_fl, accuracy_fl = fl_results

    report.plot_accuracies(history_ul, history_fl)
    report.test_set_performance(accuracy_ul, loss_ul)
    report.test_set_performance(accuracy_fl, loss_fl)


def run_uni(
    xy_train: Tuple[np.ndarray, np.ndarray],
    xy_val: Tuple[np.ndarray, np.ndarray],
    xy_test: Tuple[np.ndarray, np.ndarray],
    epochs: int,
):
    # Initialize model and participant
    model = cnn_compiled()
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
    C = 3
    # Init participants
    participants = []
    for xy_train in xy_train_partitions:
        model = cnn_compiled()
        participant = Participant(model, xy_train=xy_train, xy_val=xy_val)
        participants.append(participant)
    num_participants = len(participants)
    # Init coordinator
    model = cnn_compiled()
    controller = RandomController(num_participants, C)
    coordinator = Coordinator(controller, model, participants)
    # Train model
    history = coordinator.fit(num_rounds=rounds)
    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


def main(_):
    benchmark_gain_FashionMNIST()


if __name__ == "__main__":
    app.run(main=main)
