import math

import tensorflow as tf

from . import arch
from .resnet import resnet


def resnet_v2_20():
    model, _ = resnet(input_shape=(32, 32, 3), num_classes=10, version=2, n=2)
    return model


def resnet_v2_20_compiled(
    lr_initial: float = 0.1, momentum: float = 0.9, k: float = 0.15
) -> tf.keras.Model:
    model = resnet_v2_20()

    def exp_decay(epoch: int) -> float:
        return lr_initial * math.exp(-k * epoch)

    optimizer = tf.keras.optimizers.SGD(lr=exp_decay(0), momentum=momentum)
    model.compile(
        optimizer=optimizer, loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model


def fc_compiled() -> tf.keras.Model:
    model = tf.keras.models.Sequential(
        [
            tf.keras.layers.Flatten(input_shape=(28, 28)),
            tf.keras.layers.Dense(128, activation="relu"),
            tf.keras.layers.Dense(10, activation="softmax"),
        ]
    )
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model


def enas_cnn_compiled() -> tf.keras.Model:
    arch_str = [str(x) for x in [1, 2, 0, 3, 0, 0]]
    model = arch.build_architecture(arch.parse_arch_str(arch_str))
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model
