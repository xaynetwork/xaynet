from absl import logging

from autofl.datasets import cifar10_random_splits_10

from ..net import cnn_compiled
from .controller import RandomController, RoundRobinController
from .coordinator import Coordinator
from .participant import init_participants


def individual():
    # Load data
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train:", len(xy_splits))

    # Train independent models on each data partition
    # TODO common initialization for all participant models
    participants = init_participants(xy_splits)

    # Train each model
    for p in participants:
        p.train(epochs=2)

    # Evaluate the individual performance of each model
    for i, p in enumerate(participants):
        x_test, y_test = xy_test
        loss, accuracy = p.evaluate(x_test, y_test)
        logging.info("Participant", i, ":", loss, accuracy)


def round_robin():
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train:", len(xy_splits))

    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = init_participants(xy_splits)
    model = cnn_compiled()  # This will act as the initial model
    controller = RoundRobinController(num_participants=len(participants))
    coordinator = Coordinator(controller, model, participants)

    # Start training
    coordinator.fit(num_rounds=10)

    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    logging.info("Final loss and accuracy:", loss, accuracy)


def federated_learning():
    logging.info("\n\nStarting federated learning\n")
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train:", len(xy_splits))

    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = init_participants(xy_splits)
    model = cnn_compiled()
    controller = RandomController(num_participants=len(participants), C=3)
    coordinator = Coordinator(controller, model, participants)

    # Start training
    coordinator.fit(num_rounds=10)

    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    logging.info("\nFinal loss and accuracy:", loss, accuracy)
