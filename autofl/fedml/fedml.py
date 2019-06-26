from absl import logging

from autofl.data import cifar10_random_splits_10
from autofl.fedml import net

from .controller import RandomController, RoundRobinController
from .coordinator import Coordinator
from .participant import Participant

PARTICIPANTS = 10


def individual():
    # Load data
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()
    logging.info("Number of splits x/y train:", len(xy_splits))

    # Train independent models on each data partition
    participants = []
    for x_split, y_split in xy_splits:
        # TODO common initialization for all participants
        model = net.cnn()  # Create a new model for each participant
        participant = Participant(model, x_split, y_split)
        participants.append(participant)

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
    participants = []
    for x_split, y_split in xy_splits:
        model = net.cnn()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    model = net.cnn()  # This will act as the initial model
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
    participants = []
    for x_split, y_split in xy_splits:
        model = net.cnn()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    model = net.cnn()
    controller = RandomController(num_participants=len(participants), C=3)
    coordinator = Coordinator(controller, model, participants)

    # Start training
    coordinator.fit(num_rounds=10)

    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    logging.info("\nFinal loss and accuracy:", loss, accuracy)
