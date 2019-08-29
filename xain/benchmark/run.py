import random
import time
from typing import List, Optional, Tuple

import numpy as np
import tensorflow as tf
from absl import logging

from xain.benchmark.net import orig_cnn_compiled
from xain.datasets import load_splits
from xain.fl.coordinator import Coordinator, RandomController
from xain.fl.coordinator.aggregate import Aggregator
from xain.fl.participant import ModelProvider, Participant
from xain.helpers import storage
from xain.types import FederatedDatasetPartition, KerasHistory

from . import report

random.seed(0)
np.random.seed(1)
tf.compat.v1.set_random_seed(2)


# Default parameters for `unitary_versus_federated`
FLH_C = 0.1  # Fraction of participants used in each round of training
ROUNDS = 50  # Number of total rounds to train
FLH_E = 4  # Number of training epochs in each round
FLH_B = 64  # Batch size used by participants


# pylint: disable-msg=too-many-locals
def unitary_training(
    xy_train: FederatedDatasetPartition,
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    epochs: int,
    batch_size: int,
) -> Tuple[KerasHistory, float, float]:

    model_provider = ModelProvider(model_fn=orig_cnn_compiled)

    # Initialize model and participant
    cid = 0
    participant = Participant(
        cid,
        model_provider,
        xy_train=xy_train,
        xy_val=xy_val,
        num_classes=10,
        batch_size=batch_size,
    )
    model = model_provider.init_model()
    theta = model.get_weights()

    # Train model
    hist = participant.fit(model, epochs)

    # Evaluate final performance
    theta = model.get_weights()
    loss, acc = participant.evaluate(theta, xy_test)

    # Report results
    return hist, loss, acc


# pylint: disable-msg=too-many-locals,too-many-arguments
def federated_training(
    xy_train_partitions: List[FederatedDatasetPartition],
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    rounds: int,
    C: float,
    E: int,
    B: int,
    aggregator: Aggregator = None,
) -> Tuple[KerasHistory, List[List[KerasHistory]], float, float]:
    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.

    model_provider = ModelProvider(model_fn=orig_cnn_compiled)

    # Init participants
    participants = []
    for cid, xy_train in enumerate(xy_train_partitions):
        participant = Participant(
            str(cid), model_provider, xy_train, xy_val, num_classes=10, batch_size=B
        )
        participants.append(participant)
    num_participants = len(participants)

    # Init coordinator
    controller = RandomController(num_participants)
    coordinator = Coordinator(
        controller,
        model_provider,
        participants,
        C=C,
        E=E,
        xy_val=xy_val,
        aggregator=aggregator,
    )

    # Train model
    hist_co, hist_ps = coordinator.fit(num_rounds=rounds)

    # Evaluate final performance
    loss, acc = coordinator.evaluate(xy_test)

    # Report results
    return hist_co, hist_ps, loss, acc


def unitary_versus_federated(
    benchmark_name: str,
    dataset_name: str,
    C: float = FLH_C,
    E: int = FLH_E,
    B: int = FLH_B,
    rounds: int = ROUNDS,
):
    """
    :param C: Fraction of participants used in each round of training
    """
    logging.info(f"Starting {benchmark_name}")
    xy_train_partitions, xy_val, xy_test = load_splits(dataset_name)

    start = time.time()

    # Train CNN on a single partition ("unitary learning")
    # TODO train n models on all partitions
    partition_id = 0
    xy_train = xy_train_partitions[partition_id]
    logging.info(f"Run unitary training using partition {partition_id}")
    ul_hist, ul_loss, ul_acc = unitary_training(
        xy_train, xy_val, xy_test, epochs=rounds * E, batch_size=B
    )

    # Train CNN using federated learning on all partitions
    logging.info("Run federated learning using all partitions")
    fl_hist, _, fl_loss, fl_acc = federated_training(
        xy_train_partitions, xy_val, xy_test, rounds, C=C, E=E, B=B
    )

    end = time.time()

    # Write results JSON
    results = {
        "name": benchmark_name,
        "start": start,
        "end": end,
        "duration": end - start,
        "FLH_C": C,
        "FLH_E": E,
        "FLH_B": B,
        "ROUNDS": rounds,
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
    storage.write_json(results, fname="results.json")

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
            [i for i in range(E, len(fl_hist["val_acc"]) * E + 1, E)],
        ),
    ]
    # FIXME use different filenames for different datasets
    report.plot_accuracies(plot_data, fname="plot.png")
