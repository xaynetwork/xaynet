import random
import time
from typing import List, Optional, Tuple

import numpy as np
import tensorflow as tf
from absl import logging

from xain.benchmark.aggregation import task_accuracies
from xain.benchmark.net import load_model_fn
from xain.datasets import load_splits
from xain.fl.coordinator import Coordinator, RandomController
from xain.fl.coordinator.aggregate import Aggregator
from xain.fl.participant import ModelProvider, Participant
from xain.helpers import storage
from xain.types import FederatedDatasetPartition, KerasHistory, VolumeByClass

random.seed(0)
np.random.seed(1)
tf.compat.v1.set_random_seed(2)


# Default parameters for `unitary_versus_federated`
DEFAULT_R = 50  # Number of total rounds to train
DEFAULT_E = 4  # Number of training epochs in each round
DEFAULT_C = 0.1  # Fraction of participants used in each round of training
DEFAULT_B = 64  # Batch size used by participants


# pylint: disable-msg=too-many-locals,too-many-arguments
def unitary_training(
    model_name: str,
    xy_train: FederatedDatasetPartition,
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    E: int,
    B: int,
) -> Tuple[KerasHistory, float, float]:

    model_fn = load_model_fn(model_name)
    model_provider = ModelProvider(model_fn=model_fn)

    # Initialize model and participant
    cid = 0
    participant = Participant(
        cid,
        model_provider,
        xy_train=xy_train,
        xy_val=xy_val,
        num_classes=10,
        batch_size=B,
    )
    model = model_provider.init_model()
    theta = model.get_weights()

    # Train model
    hist = participant.fit(model, E)

    # Evaluate final performance
    theta = model.get_weights()
    loss, acc = participant.evaluate(theta, xy_test)

    # Report results
    return hist, loss, acc


# pylint: disable-msg=too-many-locals,too-many-arguments
def federated_training(
    model_name: str,
    xy_train_partitions: List[FederatedDatasetPartition],
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    R: int,
    E: int,
    C: float,
    B: int,
    aggregator: Aggregator = None,
) -> Tuple[
    KerasHistory, List[List[KerasHistory]], List[List[VolumeByClass]], float, float
]:
    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.

    model_fn = load_model_fn(model_name)
    model_provider = ModelProvider(model_fn=model_fn)

    # Init participants
    participants = []
    for cid, xy_train in enumerate(xy_train_partitions):
        participant = Participant(
            cid, model_provider, xy_train, xy_val, num_classes=10, batch_size=B
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
    hist_co, hist_ps, hist_metrics = coordinator.fit(num_rounds=R)

    # Evaluate final performance
    loss, acc = coordinator.evaluate(xy_test)

    # Report results
    return hist_co, hist_ps, hist_metrics, loss, acc


# FIXME remove
def unitary_versus_federated(
    benchmark_name: str,
    model_name: str,
    dataset_name: str,
    R: int = DEFAULT_R,
    E: int = DEFAULT_E,
    C: float = DEFAULT_C,
    B: int = DEFAULT_B,
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
        model_name, xy_train, xy_val, xy_test, E=R * E, B=B
    )

    # Train CNN using federated learning on all partitions
    logging.info("Run federated learning using all partitions")
    fl_hist, _, _, fl_loss, fl_acc = federated_training(
        model_name, xy_train_partitions, xy_val, xy_test, R=R, E=E, C=C, B=B
    )

    end = time.time()

    # Write results JSON
    results = {
        "name": benchmark_name,
        "start": start,
        "end": end,
        "duration": end - start,
        "R": R,
        "E": E,
        "C": C,
        "B": B,
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
    task_accuracies.plot(plot_data, fname="plot.png")
