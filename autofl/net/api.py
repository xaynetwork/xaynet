import math

import tensorflow as tf

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
