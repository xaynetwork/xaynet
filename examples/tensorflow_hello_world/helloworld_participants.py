import math
import multiprocessing as mp
from typing import Callable, List

import tensorflow as tf
from absl import app, flags
from tensorflow.keras.layers import Conv2D, Dense, Flatten, Input, MaxPool2D

from xain_fl.datasets import load_splits
from xain_fl.fl.coordinator import Coordinator, RandomController
from xain_fl.fl.coordinator.aggregate import FederatedAveragingAgg
from xain_fl.fl.participant import ModelProvider, Participant
from xain_fl.types import Partition

from xain_fl.grpc.participant import go

# Defining the 'task_name' flag here, to be used by the absl-py app.
FLAGS = flags.FLAGS
flags.DEFINE_string("task_name", None, "")


DATASET_NAME = "fashion-mnist-100p-iid-balanced"
"""Specifying a dataset name for this example.

We will use a partitioned version of the Fashion MNIST dataset.
Please see here: https://xainag.github.io/xain/

100p means that the dataset is split into 100 partitions, which are IID
Each partition represents the dataset a single client stores locally.
"""

NUM_CLASSES = 10
INPUT_SHAPE = (28, 28, 1)
"""Standard attributes of the Fashion MNIST dataset.
"""

R = 2
"""int: Number of global rounds the model is going to be trained for.
"""

E = 2
"""int: Number of local epochs.

Each Participant in a round will train the model with its local data for E epochs.
"""

C = 0.02
"""float: Fraction of total clients that participate in a training round.
"""

B = 10
"""int: Local batch size for a client update.
"""


def run_participants(potential_participants):
    processes = [
        mp.Process(target=go, args=(potential_participants[i], "localhost:50051")) for i in range(3)
    ]

    for p in processes:
        p.start()

    for p in processes:
        p.join()


def main(_):
    """Main function that runs in the script.

    This function fetches the data split into training partitions, validation and test
    dataset. A Keras model is then declared and compiled, potential Participants are
    initiated, as well as the Coordinator. We then call fit and evaluate on the
    Coordinator and print some basic stats as well as final performance metrics.
    """

    # Fetching the data.
    # xy_train_partitions: Each partition is the local training dataset of a single client.
    # xy_validation: Contains the global validation data (shared by all participants).
    # xy_test: Contains the global test data (shared by all participants).
    xy_train_partitions, xy_validation, xy_test = load_splits(DATASET_NAME)

    # Declaring model architecture and compiling Keras model.
    model_fn = create_and_compile_model

    # Passing model and learning rate functions to ModelProvider.
    model_provider = ModelProvider(model_fn=model_fn, lr_fn_fn=learning_rate_fn)

    # Initiating the Participant for each client.
    # At this stage they are not yet selected for training.
    potential_participants = init_participants(
        xy_train_partitions=xy_train_partitions,
        model_provider=model_provider,
        xy_validation=xy_validation,
    )

    # Printing some basic statistics.
    print_dataset_stats(xy_train_partitions, xy_validation, xy_test)

    run_participants(potential_participants)

    # Calling fit on the coordinator starts the training.
    # It prints validation metrics after R rounds of federated learning are completed.
    # validation_metrics, _, _, _ = coordinator.fit(num_rounds=R)
    # print_validation_metrics(validation_metrics)

    # Evaluating the model with the test set.
    # loss, accuracy = coordinator.evaluate(xy_test)

    # Printing final test loss and accuracy.
    # print(f"---\nTest completed!\nLoss: {loss} | Test accuracy: {accuracy}")


def learning_rate_fn(
    epoch_base: int, lr_initial: float = 0.002, k: float = 0.01
) -> Callable:
    """Specifies the learning rate function, in this case with exponential decay.

    Args:
        epoch_base: Base epoch value.
        lr_initial: Initial learning rate value.
        k: Exponential decay constant.

    Returns:
        Decayed learning rate based on epoch_optimizer.
    """

    def exp_decay(epoch_optimizer: int) -> float:
        epoch = epoch_base + epoch_optimizer
        return lr_initial * math.exp(-k * epoch)

    return exp_decay


def create_and_compile_model(epoch_base: int = 0) -> Callable[[], tf.keras.Model]:
    """Contains the model architecture and compiles it with Keras API.

    Args:
        epoch_base: Base epoch value.

    Returns:
        A compiled tf.keras.Model instance, with exponential learning rate decay.
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


def init_participants(
    xy_train_partitions: List[Partition],
    model_provider: ModelProvider,
    xy_validation: Partition,
) -> List[Participant]:
    """Initiates potential Participants.

    Iterate through each partition (a client's training data) and
    initiates a Participant instance.

    Args:
        xy_train_partitions: Partitioned training data.
        model_provider: ModelProvider instance holding model and
            learning rate functions.
        xy_validation: Validation dataset.

    Returns:
        A list of initiated Participant instances, one per client.
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


def print_dataset_stats(
    xy_train_partitions: List[Partition], xy_validation: Partition, xy_test: Partition
):
    """Prints some preliminary stats to the terminal.

    Args:
        xy_train_partitions: Partitioned training data.
        xy_validation: Validation dataset.
        xy_test: Test dataset.
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


def print_validation_metrics(validation_metrics: dict):
    """Prints the validation metrics to the terminal.

    Args:
        validation_metrics: Dictionary containing loss and accuracy per each round.
    """

    for round_id in range(R):
        print(
            f"validation round: {round_id + 1}: "
            f"loss: {validation_metrics['val_loss'][round_id]}, "
            f"accuracy: {validation_metrics['val_acc'][round_id]}"
        )


if __name__ == "__main__":
    app.run(main=main)
