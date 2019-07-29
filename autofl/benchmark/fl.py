from typing import List, Tuple

import numpy as np
from absl import app, logging

from autofl.datasets import (
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
    fashion_mnist_100p_IID_balanced,
)
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.net import orig_cnn_compiled

from . import report

FLH_B = 32  # Batch size used by participants
FLH_E = 1  # Number of training episodes in each round
FLH_C = 0.1  # Fraction of participants used in each round of training

ROUNDS = 40


def benchmark_ul_fl_FashionMNIST_100p_IID_balanced():
    logging.info("Starting Fashion-MNIST-100p-IID-balanced Benchmark")
    xy_parts, xy_val, xy_test = fashion_mnist_100p_IID_balanced.load_splits()
    print("Length xy_parts:", len(xy_parts))
    run_unitary_versus_federated(xy_parts, xy_val, xy_test, C=0.1)


def benchmark_ul_fl_FashionMNIST_10p_0():
    logging.info("Starting Fashion-MNIST-10p-0 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_600.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def benchmark_ul_fl_FashionMNIST_10p_1000():
    logging.info("Starting Fashion-MNIST-10p-1000 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_500_1k_bias.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def benchmark_ul_fl_FashionMNIST_10p_5400():
    logging.info("Starting Fashion-MNIST-10p-5400 Benchmark")
    xy_splits, xy_val, xy_test = fashion_mnist_10s_single_class.load_splits()
    run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def run_unitary_versus_federated(xy_splits, xy_val, xy_test, C):
    # TODO initialize both (UL|FL) models the same way

    # Train CNN on a single partition ("unitary learning")
    # TODO train n models on all partitions
    partition_id = 0
    logging.info("> Train model on partition {}".format(partition_id))
    ul_results = run_uni(
        xy_splits[partition_id], xy_val, xy_test, epochs=ROUNDS, B=FLH_B
    )

    # Train CNN using federated learning on all partitions
    logging.info("> Train federated model on all partitions")
    fl_results = run_fed(xy_splits, xy_val, xy_test, ROUNDS, C=C, E=FLH_E, B=FLH_B)

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
    B: int,
):
    # Initialize model and participant
    model = orig_cnn_compiled()
    participant = Participant(
        model, xy_train=xy_train, xy_val=xy_val, num_classes=10, batch_size=B
    )
    # Train model
    history = participant._train(epochs)  # pylint: disable-msg=protected-access
    # Evaluate final performance
    loss, accuracy = participant.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


# pylint: disable-msg=too-many-locals,too-many-arguments
def run_fed(
    xy_train_partitions: List[Tuple[np.ndarray, np.ndarray]],
    xy_val: Tuple[np.ndarray, np.ndarray],
    xy_test: Tuple[np.ndarray, np.ndarray],
    rounds: int,
    C: float,
    E: int,
    B: int,
):
    # Init participants
    participants = []
    for xy_train in xy_train_partitions:
        model = orig_cnn_compiled()
        participant = Participant(model, xy_train, xy_val, num_classes=10, batch_size=B)
        participants.append(participant)
    num_participants = len(participants)
    # Init coordinator
    model = orig_cnn_compiled()
    controller = RandomController(num_participants)
    coordinator = Coordinator(controller, model, participants, C=C, E=E)
    # Train model
    history = coordinator.fit(num_rounds=rounds)
    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


def main(_):
    # benchmark_ul_fl_FashionMNIST_10p_0()
    # benchmark_ul_fl_FashionMNIST_10p_1000()
    # benchmark_ul_fl_FashionMNIST_10p_5400()
    benchmark_ul_fl_FashionMNIST_100p_IID_balanced()


if __name__ == "__main__":
    app.run(main=main)
