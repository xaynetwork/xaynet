from absl import logging

from autofl.data import cifar10_random_splits_10
from autofl.fedml import Coordinator, Participant, RandomController
from autofl.net import resnet_v2_20_compiled


def main(_):
    eval_CIFAR_10_with_random_controller()


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
    x_test, y_test = xy_test
    loss, accuracy = coordinator.evaluate(x_test, y_test)
    logging.info("Final loss: {}, accuracy: {}".format(loss, accuracy))
