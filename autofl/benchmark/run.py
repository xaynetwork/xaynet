import random
from typing import Dict, List, Tuple

import numpy as np
import tensorflow as tf

from autofl.fl.coordinator import Coordinator, RandomController
from autofl.fl.coordinator.aggregate import Aggregator
from autofl.fl.participant import ModelProvider, Participant
from autofl.net import orig_cnn_compiled
from autofl.types import FederatedDatasetPartition

random.seed(0)
np.random.seed(1)
tf.compat.v1.set_random_seed(2)

# pylint: disable-msg=too-many-locals
def unitary_training(
    xy_train: FederatedDatasetPartition,
    xy_val: FederatedDatasetPartition,
    xy_test: FederatedDatasetPartition,
    epochs: int,
    batch_size: int,
) -> Tuple[Dict[str, List[float]], float, float]:

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

    # Evaluate initial training and validation set loss (and accuracy)
    train_loss, train_acc = participant.evaluate(
        theta, xy_train
    )  # Note: This evaluates just one batch, not the entire dataset
    val_loss, val_acc = participant.evaluate(theta, xy_val)

    # Train model
    history = participant._train(model, epochs)  # pylint: disable-msg=protected-access
    history = {
        "acc": [float(train_acc)] + history["acc"],
        "loss": [float(train_loss)] + history["loss"],
        "val_acc": [float(val_acc)] + history["val_acc"],
        "val_loss": [float(val_loss)] + history["val_loss"],
    }

    # Evaluate final performance
    theta = model.get_weights()
    loss, accuracy = participant.evaluate(theta, xy_test)

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
    history = coordinator.fit(num_rounds=rounds)

    # Evaluate final performance
    loss, accuracy = coordinator.evaluate(xy_test)

    # Report results
    return history, loss, accuracy
