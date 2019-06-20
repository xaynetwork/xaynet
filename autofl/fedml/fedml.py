from autofl.data import data
from autofl.fedml import net

from .coordinator import Coordinator
from .participant import Participant

PARTICIPANTS = 10


def main():
    federated_learning()


def individual():
    # Load data
    xy_splits, xy_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
    print("Number of splits x/y train:", len(xy_splits))
    # Create model
    model = net.fc()
    model.summary()
    # Train independent models on each data partition
    ps = []
    for x_split, y_split in xy_splits:
        model = net.fc()  # Create a new model for each participant
        participant = Participant(model, x_split, y_split)
        ps.append(participant)
    # Train each model
    for p in ps:
        p.train(epochs=2)
    # Evaluate the individual performance of each model
    for i, p in enumerate(ps):
        x_test, y_test = xy_test
        loss, accuracy = p.evaluate(x_test, y_test)
        print("Participant", i, ":", loss, accuracy)


def round_robin():
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
    print("Number of splits x/y train:", len(xy_splits))
    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = []
    for x_split, y_split in xy_splits:
        model = net.fc()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    model = net.fc()  # This will act as the initial model
    coordinator = Coordinator(model, participants)
    # Start training
    coordinator.train(10)
    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    print("Final loss and accuracy:", loss, accuracy)


def federated_learning():
    print("\n\nStarting federated learning\n")
    # Load data (multiple splits for training and one split for validation)
    xy_splits, xy_test = data.load_splits_mnist(num_splits=PARTICIPANTS)
    print("Number of splits x/y train:", len(xy_splits))
    # Initialize participants and coordinator
    # Note that there is no need for common initialization at this point: Common
    # initialization will happen during the first few rounds because the coordinator will
    # push its own weight to the respective participants of each training round.
    participants = []
    for x_split, y_split in xy_splits:
        model = net.fc()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)
    model = net.fc()  # This will act as the initial model
    coordinator = Coordinator(model, participants)
    # Start training
    coordinator.train_fl(10, C=3)
    # Evaluate final model
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    print("\nFinal loss and accuracy:", loss, accuracy)
