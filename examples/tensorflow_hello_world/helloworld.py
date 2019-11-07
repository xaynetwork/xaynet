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
# This comes from the Fashion MNIST dataset:
# https://research.zalando.com/welcome/mission/research-projects/fashion-mnist/
# 100p means that it is split into 100 partitions, which are IID
# Each partition represents the dataset a single client stores locally
DATASET_NAME = "fashion-mnist-100p-iid-balanced"

# standard attributes of the Fashion MNIST dataset
NUM_CLASSES = 10
INPUT_SHAPE = (28, 28, 1)

# R is the number of global epochs the model is going to be trained for
R = 2

# E is the number of local epochs
# Each Participant in a round will train the model with its local data for E epochs
E = 2

# C is the fraction of total clients that participate in a training round
C = 0.02

# B is the local batch size for a client update
B = 10

# default values for exponential decay learning rate
DEFAULT_LR = 0.002
DEFAULT_K = 0.01


def main(_):
    # fetch the data
    # xy_train_partitions: each represent one local training data on a client
    # xy_validation contains the global validation data (used for each participant)
    # xy_test contains the global test data
    xy_train_partitions, xy_validation, xy_test = load_splits(DATASET_NAME)

    # declare model and compile Keras model
    model_fn = create_and_compile_model

    # pass model and learning rate functions to ModelProvider
    model_provider = ModelProvider(model_fn=model_fn, lr_fn_fn=learning_rate_fn)

    # init the Participant for each client.
    # At this stage they are not yet selected for training
    potential_participants = init_participants(
        xy_train_partitions=xy_train_partitions,
        model_provider=model_provider,
        xy_validation=xy_validation,
    )

    # init the centralized Coordinator of the training
    coordinator = init_coordinator(
        participants=potential_participants,
        xy_train_partitions=xy_train_partitions,
        model_provider=model_provider,
        xy_validation=xy_validation,
    )

    # printing some basic statistics
    print_dataset_stats(xy_train_partitions, xy_validation, xy_test)

    # calling fit on the coordinator starts the training
    # and print validation metrics after each global round
    validation_metrics, _, _, _ = coordinator.fit(num_rounds=R)
    print_validation_metrics(validation_metrics)

    # evaluate the model with the test set
    loss, accuracy = coordinator.evaluate(xy_test)
    print(f"---\nTest completed!\nLoss: {loss} | Test accuracy: {accuracy}")


# specify learning rate function, in this case with exponential decay
def learning_rate_fn(epoch_base, lr_initial=DEFAULT_LR, k=DEFAULT_K):
    def exp_decay(epoch_optimizer: int) -> float:
        epoch = epoch_base + epoch_optimizer
        return lr_initial * math.exp(-k * epoch)

    return exp_decay


# this function contains the model architecture
def create_and_compile_model(epoch_base=0):
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

    # Architecture
    inputs = Input(shape=INPUT_SHAPE)
    x = add_convolution(filters=32, kernel_inizializer=ki)(inputs)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = add_convolution(filters=64, kernel_inizializer=ki)(x)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = Flatten()(x)
    x = Dense(512, kernel_initializer=ki, activation="relu")(x)
    outputs = Dense(NUM_CLASSES, kernel_initializer=ki, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    # Compile model with exponential learning rate decay
    lr_fn = learning_rate_fn(epoch_base=epoch_base)
    optimizer = tf.keras.optimizers.Adam(lr=lr_fn(0))

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=optimizer,
        metrics=["accuracy"],
    )
    return model


# iterate through each partition (a client's training data)
# and init a Participant class
def init_participants(xy_train_partitions, model_provider, xy_validation):
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


# init the Coordinator
def init_coordinator(participants, xy_train_partitions, model_provider, xy_validation):

    # the controller will select the indices of the clients participating in the training
    # right now it's initialized with all the indices of the clients available
    num_clients = len(participants)
    assert num_clients == len(xy_train_partitions)
    controller = RandomController(num_clients)

    # init the aggregator, Federated Averaging is the state-of-the-art aggregator
    # see here for more details: https://arxiv.org/pdf/1602.05629.pdf
    aggregator = FederatedAveragingAgg()

    # the Coordinator will coordinate the training. it will:
    #   - select a C fraction of potential participants
    #   - send training jobs to each selected Participant
    #       who will train its own local data for E local epochs
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


# printing some preliminary stats to the terminal
def print_dataset_stats(xy_train_partitions, xy_validation, xy_test):
    print(f"\nthere are {len(xy_train_partitions)} client/potential partitions")
    images_first_client, labels_first_client = xy_train_partitions[0]
    n_images, height, width = images_first_client.shape
    print(f"the first client has {n_images} images, of {width}x{height} size")
    print(f"there are {len(labels_first_client)} labels, one label per image")

    validation_images, _ = xy_validation
    print(f"---\nthe validation set is made of {len(validation_images)} images")

    test_images, _ = xy_test
    print(f"the test set is made of {len(test_images)} images\n---")


# printing the validation metrics to the terminal
def print_validation_metrics(validation_metrics):
    for round_id in range(R):
        print(
            f"validation round: {round_id + 1}: "
            f"loss: {validation_metrics['val_loss'][round_id]}, "
            f"accuracy: {validation_metrics['val_acc'][round_id]}"
        )


if __name__ == "__main__":
    app.run(main=main)
