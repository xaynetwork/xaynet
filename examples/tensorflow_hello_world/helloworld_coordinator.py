import os
import math
from typing import Callable, List

import tensorflow as tf
from absl import app, flags
from tensorflow.keras.layers import Conv2D, Dense, Flatten, Input, MaxPool2D

from xain.datasets import load_splits
from xain.fl.coordinator import Coordinator, RandomController
from xain.fl.coordinator.aggregate import FederatedAveragingAgg
from xain.fl.participant import ModelProvider, Participant
from xain.types import Partition

from xain.grpc.coordinator import Coordinator, serve

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

C = 1
"""float: Fraction of total clients that participate in a training round.
"""


def main():
    model = create_and_compile_model()
    theta = model.get_weights()
    coordinator = Coordinator(
        num_rounds=R, required_participants=3 * C, theta=theta, epochs=E
    )

    serve(coordinator)


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


if __name__ == "__main__":
    main()
