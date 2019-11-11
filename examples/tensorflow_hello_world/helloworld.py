import math

import tensorflow as tf
from absl import app, flags
from tensorflow.keras.layers import Conv2D, Dense, Flatten, Input, MaxPool2D

from xain.datasets import load_splits
from xain.fl.coordinator import Coordinator, RandomController
from xain.fl.coordinator.aggregate import FederatedAveragingAgg
from xain.fl.participant import ModelProvider, Participant

# TODO: this feels a bit hacky. it's the only flag that needs to be added here.
#  any better solution?
FLAGS = flags.FLAGS
flags.DEFINE_string("task_name", None, "")


# We first define some constants for this specific example

# Specify a dataset name for this example
# We will use a partitioned version of the Fashion MNIST dataset
# Please see here: https://xainag.github.io/xain/

# 100p means that the dataset is split into 100 partitions, which are IID
# Each partition represents the dataset a single client stores locally
DATASET_NAME = "fashion-mnist-100p-iid-balanced"

# Standard attributes of the Fashion MNIST dataset
NUM_CLASSES = 10
INPUT_SHAPE = (28, 28, 1)

# R is the number of global rounds the model is going to be trained for
R = 2

# E is the number of local epochs
# Each Participant in a round will train the model with its local data for E epochs
E = 2

# C is the fraction of total clients that participate in a training round
C = 0.02

# B is the local batch size for a client update
B = 10


def main(_):
    # Fetch the data
    # xy_train_partitions: Each partition represents the local training dataset of a single client
    # xy_validation: Contains the global validation data (shared by all participants)
    # xy_test: Contains the global test data (shared by all participants)
    xy_train_partitions, xy_validation, xy_test = load_splits(DATASET_NAME)

    # Declare model architecture and compile Keras model
    model_fn = create_and_compile_model

    # Pass model and learning rate functions to ModelProvider
    model_provider = ModelProvider(model_fn=model_fn, lr_fn_fn=learning_rate_fn)

    # Init the Participant for each client.
    # At this stage they are not yet selected for training
    potential_participants = init_participants(
        xy_train_partitions=xy_train_partitions,
        model_provider=model_provider,
        xy_validation=xy_validation,
    )

    # Init the centralized Coordinator of the training
    coordinator = init_coordinator(
        participants=potential_participants,
        model_provider=model_provider,
        xy_validation=xy_validation,
    )

    # Printing some basic statistics
    print_dataset_stats(xy_train_partitions, xy_validation, xy_test)

    # Calling fit on the coordinator starts the training
    # and prints validation metrics after R rounds of federated learning are completed
    validation_metrics, _, _, _ = coordinator.fit(num_rounds=R)
    print_validation_metrics(validation_metrics)

    # Evaluate the model with the test set
    loss, accuracy = coordinator.evaluate(xy_test)

    # Print final test loss and accuracy
    print(f"---\nTest completed!\nLoss: {loss} | Test accuracy: {accuracy}")


def learning_rate_fn(epoch_base, lr_initial=0.002, k=0.01):
    """
    Specify the learning rate function, in this case with exponential decay.

    :param epoch_base: base epoch value
    :param lr_initial: initial learning rate value
    :param k: exponential decay constant
    :return: decayed learning rate based on the epoch_optimizer
    """

    def exp_decay(epoch_optimizer: int) -> float:
        epoch = epoch_base + epoch_optimizer
        return lr_initial * math.exp(-k * epoch)

    return exp_decay


def create_and_compile_model(epoch_base=0):
    """
    This function contains the model architecture.
    Once the architecture is specified, the model is compiled.

    :param epoch_base: base epoch value
    :return: a compiled tf.keras.Model instance, with exponential learning rate decay
    """

    def add_convolution(filters, kernel_inizializer):
        convolution = Conv2D(
            filters,
            kernel_size=(5, 5),
            strides=(1, 1),
            kernel_initializer=kernel_inizializer,
            padding="same",
            activation="relu",
        )
        return convolution

    ki = tf.keras.initializers.glorot_uniform(seed=42)

    inputs = Input(shape=INPUT_SHAPE)
    x = add_convolution(filters=32, kernel_inizializer=ki)(inputs)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = add_convolution(filters=64, kernel_inizializer=ki)(x)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = Flatten()(x)
    x = Dense(512, kernel_initializer=ki, activation="relu")(x)
    outputs = Dense(NUM_CLASSES, kernel_initializer=ki, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    lr_fn = learning_rate_fn(epoch_base=epoch_base)
    optimizer = tf.keras.optimizers.Adam(lr=lr_fn(0))

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=optimizer,
        metrics=["accuracy"],
    )
    return model


def init_participants(xy_train_partitions, model_provider, xy_validation):
    """
    Iterate through each partition (a client's training data) and init a Participant class.

    :param xy_train_partitions: partitioned training data
    :param model_provider: ModelProvider instance holding model and learning rate functions
    :param xy_validation: validation dataset
    :return: list of initiated Participant instances, one per client
    """
    potential_participants = []
    for client_id, xy_train in enumerate(xy_train_partitions):
        participant = Participant(
            cid=client_id,
            model_provider=model_provider,
            xy_train=xy_train,
            xy_val=xy_validation,
            num_classes=NUM_CLASSES,
            batch_size=B,
        )
        potential_participants.append(participant)
    return potential_participants


def init_coordinator(participants, model_provider, xy_validation):
    """
    This function initiates the Coordinator.

    The controller will select the indices of the clients participating in the training.
    It is initialized with all the indices of the clients available.

    Federated Averaging is a common aggregation method.
    See here for more details: https://arxiv.org/pdf/1602.05629.pdf

    The Coordinator will coordinate the training. It will:
    - Select a C fraction of potential participants.
    - Send training jobs to each selected Participant,
    who will train its own local data for E local epochs.

    :param participants: list of initiated Participant instances, one per client
    :param model_provider: ModelProvider instance holding model and learning rate functions
    :param xy_validation: validation dataset
    :return: initiated Coordinator instance
    """
    num_clients = len(participants)
    controller = RandomController(num_clients)

    aggregator = FederatedAveragingAgg()

    coordinator = Coordinator(
        controller=controller,
        model_provider=model_provider,
        participants=participants,
        C=C,
        E=E,
        xy_val=xy_validation,
        aggregator=aggregator,
    )
    return coordinator


def print_dataset_stats(xy_train_partitions, xy_validation, xy_test):
    """
    This function prints some preliminary stats to the terminal.

    :param xy_train_partitions: partitioned training data
    :param xy_validation: validation dataset
    :param xy_test: test dataset
    :return: None
    """
    print(f"\nthere are {len(xy_train_partitions)} client/potential partitions")
    images_first_client, labels_first_client = xy_train_partitions[0]
    n_images, height, width = images_first_client.shape
    print(f"the first client has {n_images} images, of {width}x{height} size")
    print(f"there are {len(labels_first_client)} labels, one label per image")

    validation_images, _ = xy_validation
    print(f"---\nthe validation set is made of {len(validation_images)} images")

    test_images, _ = xy_test
    print(f"the test set is made of {len(test_images)} images\n---")


def print_validation_metrics(validation_metrics):
    """
    Printing the validation metrics to the terminal.

    :param validation_metrics: dictionary containing loss and accuracy per each round
    :return: None
    """
    for round_id in range(R):
        print(
            f"validation round: {round_id + 1}: "
            f"loss: {validation_metrics['val_loss'][round_id]}, "
            f"accuracy: {validation_metrics['val_acc'][round_id]}"
        )


if __name__ == "__main__":
    app.run(main=main)
