from absl import logging

from autofl.datasets import cifar10_random_splits_10
from autofl.fedml import Coordinator, RandomController, init_participants, net


def main(_):
    train_CIFAR_10_with_random_controller()


# pytest-disable: duplicate-code
def train_CIFAR_10_with_random_controller():
    """
    - Data: CIFAR-10, 10 shards, each shard: 500 examples for each class (perfectly balanced)
    - Model: ResNet20v2
    - Controller: RandomController
    """
    logging.info("Starting benchmark")

    # Init data
    # TODO load perfectly balanced data
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()

    # Init participants
    participants = init_participants(xy_splits)

    # Init controller
    controller = RandomController(len(participants), C=3)

    # Init model
    # TODO replace with ResNet
    model = net.cnn()

    # Init coordinator
    coordinator = Coordinator(controller, model, participants)

    # Train
    coordinator.fit(num_rounds=40)

    # Evaluate
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    logging.info("Final loss: {}, accuracy: {}".format(loss, accuracy))
