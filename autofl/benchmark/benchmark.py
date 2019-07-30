from absl import logging

from autofl.datasets import cifar10_random_splits_10, fashion_mnist_10s_600
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.net import orig_cnn_compiled, resnet20v2_compiled


def main(_):
    eval_MNIST()


def eval_MNIST():
    fn_name = eval_MNIST.__name__
    logging.info("Starting benchmark: {}".format(fn_name))

    # Init data
    # TODO load perfectly balanced splits
    # TODO: use xy_val
    xy_splits, xy_val, xy_test = (  # pylint: disable=unused-variable
        fashion_mnist_10s_600.load_splits()
    )

    # Init participants
    participants = []
    for x_split, y_split in xy_splits:
        model = orig_cnn_compiled()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)

    # Init coordinator
    controller = RandomController(len(participants))
    model = orig_cnn_compiled()
    coordinator = Coordinator(controller, model, participants, C=0.3)

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
    xy_splits, xy_val, xy_test = (  # pylint: disable=unused-variable
        cifar10_random_splits_10.load_splits()
    )

    # Init participants
    participants = []
    for x_split, y_split in xy_splits:
        model = resnet20v2_compiled()
        participant = Participant(model, x_split, y_split)
        participants.append(participant)

    # Init controller
    controller = RandomController(len(participants))

    # Init model
    model = resnet20v2_compiled()

    # Init coordinator
    coordinator = Coordinator(controller, model, participants, C=0.3)

    # Train
    coordinator.fit(num_rounds=40)

    # Evaluate
    loss, accuracy = coordinator.evaluate(xy_test)
    logging.info("Final loss: {}, accuracy: {}".format(loss, accuracy))
