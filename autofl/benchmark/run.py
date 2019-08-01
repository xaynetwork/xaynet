import random
from typing import List

import numpy as np
import tensorflow as tf

from autofl.fedml import Coordinator, Participant, RandomController
from autofl.fedml.aggregate import Aggregator
from autofl.net import orig_cnn_compiled
from autofl.types import FederatedDatasetPartition

random.seed(0)
np.random.seed(1)
tf.set_random_seed(2)

MODEL_SEED = 1096


def unitary_training(
    xy_train: FederatedDatasetPartition,
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    epochs: int,
    batch_size: int,
):
    # Initialize model and participant
    model = orig_cnn_compiled(seed=MODEL_SEED)
    participant = Participant(
        model, xy_train=xy_train, xy_val=xy_val, num_classes=10, batch_size=batch_size
    )
    # Train model
    history = participant._train(epochs)  # pylint: disable-msg=protected-access
    # Evaluate final performance
    loss, accuracy = participant.evaluate(xy_test)
    # Report results
    return history, loss, accuracy


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
):
    # Init participants
    participants = []
    for xy_train in xy_train_partitions:
        model = orig_cnn_compiled(seed=MODEL_SEED)
        participant = Participant(model, xy_train, xy_val, num_classes=10, batch_size=B)
        participants.append(participant)
    num_participants = len(participants)

    # Init coordinator
    model = orig_cnn_compiled(seed=MODEL_SEED)
    controller = RandomController(num_participants)
    coordinator = Coordinator(
        controller, model, participants, C=C, E=E, aggregator=aggregator
    )

    # Train model
    history = coordinator.fit(num_rounds=rounds)
    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)
    # Report results
    return history, loss, accuracy
