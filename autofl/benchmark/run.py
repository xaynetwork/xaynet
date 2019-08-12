import random
from typing import Dict, List, Tuple

import numpy as np
import tensorflow as tf

from autofl.fedml import Coordinator, Participant, RandomController
from autofl.fedml.aggregate import Aggregator
from autofl.net import orig_cnn_compiled
from autofl.types import FederatedDatasetPartition

random.seed(0)
np.random.seed(1)
tf.compat.v1.set_random_seed(2)

MODEL_SEED = 1096


def unitary_training(
    xy_train: FederatedDatasetPartition,
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    epochs: int,
    batch_size: int,
) -> Tuple[Dict[str, List[float]], float, float]:
    # Initialize model and participant
    model = orig_cnn_compiled(seed=MODEL_SEED)
    participant = Participant(
        model, xy_train=xy_train, xy_val=xy_val, num_classes=10, batch_size=batch_size
    )
    # Train model
    train_loss, train_acc = participant.evaluate(xy_train)  # Note: Just one batch
    val_loss, val_acc = participant.evaluate(xy_val)
    history = participant._train(epochs)  # pylint: disable-msg=protected-access
    history = {
        "acc": [float(train_acc)] + history["acc"],
        "loss": [float(train_loss)] + history["loss"],
        "val_acc": [float(val_acc)] + history["val_acc"],
        "val_loss": [float(val_loss)] + history["val_loss"],
    }
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
) -> Tuple[Dict[str, List[float]], float, float]:
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
        controller, model, participants, C=C, E=E, xy_val=xy_val, aggregator=aggregator
    )

    # Train model
    history = coordinator.fit(num_rounds=rounds)
    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)
    # Report results
    return history, loss, accuracy
