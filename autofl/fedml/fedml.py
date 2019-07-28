from absl import logging

from autofl.datasets import cifar10_random_splits_10
from autofl.net import orig_cnn_compiled

from .controller import RandomController, RoundRobinController
from .coordinator import Coordinator
from .participant import init_participants


def individual(_):
    # Load data
    xy_splits, xy_val, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train: {}".format(len(xy_splits)))

    # Train independent models on each data partition
    # TODO common initialization for all participant models
    participants = init_participants(xy_splits, xy_val)

    # Train each model
    for p in participants:
        p.train(epochs=2)

    # Evaluate the individual performance of each model
    for i, p in enumerate(participants):
        loss, accuracy = p.evaluate(xy_test)
        logging.info("Participant {}: {}, {}".format(i, loss, accuracy))


def round_robin(_):
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_val, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train: {}".format(len(xy_splits)))

    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = init_participants(xy_splits, xy_val)
    model = orig_cnn_compiled()  # This will act as the initial model
    controller = RoundRobinController(num_participants=len(participants))
    coordinator = Coordinator(controller, model, participants, C=0.1)

    # Start training
    coordinator.fit(num_rounds=10)

    # Evaluate final model
    loss, accuracy = coordinator.evaluate(xy_test)
    logging.info("\nFinal loss {}, accuracy {}".format(loss, accuracy))


def federated_learning(_):
    logging.info("\n\nStarting federated learning\n")
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_val, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train: {}".format(len(xy_splits)))

    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = init_participants(xy_splits, xy_val)
    model = orig_cnn_compiled()
    controller = RandomController(num_participants=len(participants))
    coordinator = Coordinator(controller, model, participants, C=0.3)

    # Start training
    coordinator.fit(num_rounds=10)

    # Evaluate final model
    loss, accuracy = coordinator.evaluate(xy_test)
    logging.info("\nFinal loss {}, accuracy {}".format(loss, accuracy))
