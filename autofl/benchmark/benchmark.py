from absl import logging

from autofl.datasets import cifar10_random_splits_10
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.generator import data
from autofl.net import fc_compiled, resnet_v2_20_compiled


def main(_):
    eval_MNIST()


def eval_MNIST():
    fn_name = eval_MNIST.__name__
    logging.info("Starting benchmark: {}".format(fn_name))

    # Init data
    # TODO load perfectly balanced splits
    xy_splits, xy_test = data.generate_splits_mnist(num_splits=10)

    # Init participants
    participants = []
    for x_split, y_split in xy_splits:
        model = fc_compiled()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)

    # Init coordinator
    controller = RandomController(len(participants), C=3)
    model = fc_compiled()
    coordinator = Coordinator(controller, model, participants)

    # Train
    coordinator.fit(num_rounds=2)

    # Evaluate
    loss, accuracy = coordinator.evaluate(xy_test)
    logging.info("Final loss: {}, accuracy: {}".format(loss, accuracy))


def eval_CIFAR_10_centralized():
    fn_name = eval_CIFAR_10_centralized.__name__
    logging.info("Starting benchmark: {}".format(fn_name))


# pytest-disable: duplicate-code
def eval_CIFAR_10_with_random_controller():
    """
    - Data: CIFAR-10, 10 shards, each shard: 500 examples for each class (perfectly balanced)
    - Model: ResNet20v2
    - Controller: RandomController
    """
    fn_name = eval_CIFAR_10_centralized.__name__
    logging.info("Starting benchmark: {}".format(fn_name))

    # Init data
    # TODO load perfectly balanced data
    xy_splits, xy_test = cifar10_random_splits_10.load_splits()

    # Init participants
    participants = []
    for x_split, y_split in xy_splits:
        model = resnet_v2_20_compiled()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)

    # Init controller
    controller = RandomController(len(participants), C=3)

    # Init model
    model = resnet_v2_20_compiled()

    # Init coordinator
    coordinator = Coordinator(controller, model, participants)

    # Train
    coordinator.fit(num_rounds=40)

    # Evaluate
    loss, accuracy = coordinator.evaluate(xy_test)
    logging.info("Final loss: {}, accuracy: {}".format(loss, accuracy))
