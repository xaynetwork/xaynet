from typing import Callable

import tensorflow as tf
from tensorflow.keras.layers import Conv2D, Dense, Dropout, Flatten, MaxPool2D

from . import lr_schedule

# Derived from:
# https://medium.com/tensorflow/hello-deep-learning-fashion-mnist-with-keras-50fcff8cd74a

DEFAULT_LR = 0.001
DEFAULT_K = 0.025


# pylint: disable-msg=unused-argument
def blog_cnn_compiled(
    input_shape=(28, 28, 1),
    num_classes: int = 10,
    lr_initial: float = DEFAULT_LR,
    k: float = DEFAULT_K,
    momentum: float = 0.9,
    epoch_base: int = 0,
    seed: int = 2017,
) -> tf.keras.Model:
    ki = tf.keras.initializers.glorot_uniform(seed=seed)

    model = tf.keras.Sequential()
    # Must define the input shape in the first layer of the neural network
    model.add(
        Conv2D(
            filters=64,
            kernel_size=2,
            padding="same",
            activation="relu",
            kernel_initializer=ki,
            input_shape=input_shape,
        )
    )
    model.add(MaxPool2D(pool_size=2))
    model.add(Dropout(0.3))
    model.add(
        Conv2D(
            filters=32,
            kernel_size=2,
            padding="same",
            activation="relu",
            kernel_initializer=ki,
        )
    )
    model.add(MaxPool2D(pool_size=2))
    model.add(Dropout(0.3))
    model.add(Flatten())
    model.add(Dense(256, activation="relu", kernel_initializer=ki))
    model.add(Dropout(0.5))
    model.add(Dense(num_classes, activation="softmax", kernel_initializer=ki))

    # Compile model with exponential learning rate decay
    lr_fn = blog_cnn_lr_fn(epoch_base=epoch_base)
    optimizer = tf.keras.optimizers.Adam(lr=lr_fn(0))

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=optimizer,
        metrics=["accuracy"],
    )
    return model


def blog_cnn_lr_fn(
    epoch_base: int, lr_initial: float = DEFAULT_LR, k: float = DEFAULT_K
) -> Callable:
    return lr_schedule.exp_decay_fn(epoch_base=epoch_base, lr_initial=lr_initial, k=k)
