import math

import tensorflow as tf
from tensorflow.keras.layers import Conv2D, Dense, Dropout, Flatten, Input, MaxPool2D

from .resnet import resnet


def resnet_v2_20_compiled(
    lr_initial: float = 0.1, momentum: float = 0.9, k: float = 0.15
) -> tf.keras.Model:
    model, _ = resnet(input_shape=(32, 32, 3), num_classes=10, version=2, n=2)

    def exp_decay(epoch: int) -> float:
        return lr_initial * math.exp(-k * epoch)

    optimizer = tf.keras.optimizers.SGD(lr=exp_decay(0), momentum=momentum)
    model.compile(
        optimizer=optimizer, loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model


def fc_compiled(input_shape=(28, 28, 1), num_classes=10) -> tf.keras.Model:
    inputs = Input(shape=input_shape)
    x = Flatten()(inputs)
    x = Dense(128, activation="relu")(x)
    outputs = Dense(num_classes, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )
    return model


def cnn_compiled(input_shape=(28, 28, 1), num_classes=10) -> tf.keras.Model:
    inputs = Input(shape=input_shape)
    x = Conv2D(32, kernel_size=3, activation="relu")(inputs)
    x = Conv2D(64, kernel_size=3, activation="relu")(x)
    x = MaxPool2D(pool_size=(2, 2))(x)
    x = Dropout(0.25)(x)
    x = Flatten()(x)
    x = Dense(128, activation="relu")(x)
    x = Dropout(0.5)(x)
    outputs = Dense(num_classes, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )
    return model
