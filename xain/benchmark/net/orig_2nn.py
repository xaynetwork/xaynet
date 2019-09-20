from typing import Callable

import tensorflow as tf
from tensorflow.keras.layers import Dense, Flatten, Input

from . import lr_schedule

DEFAULT_LR = 0.01
DEFAULT_K = 0.025


# pylint: disable-msg=unused-argument
def orig_2nn_compiled(
    input_shape=(28, 28, 1),
    num_classes=10,
    lr_initial: float = DEFAULT_LR,
    k: float = DEFAULT_K,
    momentum: float = 0.0,
    epoch_base: int = 0,
    seed: int = 2017,
) -> tf.keras.Model:
    # Kernel initializer
    ki = tf.keras.initializers.glorot_uniform(seed=seed)

    # Architecture
    inputs = Input(shape=input_shape)
    x = Flatten()(inputs)
    x = Dense(200, kernel_initializer=ki, activation="relu")(x)
    x = Dense(200, kernel_initializer=ki, activation="relu")(x)
    outputs = Dense(num_classes, kernel_initializer=ki, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    # Compile model with exponential learning rate decay
    lr_fn = orig_2nn_lr_fn(epoch_base=epoch_base)
    optimizer = tf.keras.optimizers.SGD(lr=lr_fn(0), momentum=momentum)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=optimizer,
        metrics=["accuracy"],
    )
    return model


def orig_2nn_lr_fn(
    epoch_base: int, lr_initial: float = DEFAULT_LR, k: float = DEFAULT_K
) -> Callable:
    return lr_schedule.exp_decay_fn(epoch_base=epoch_base, lr_initial=lr_initial, k=k)
