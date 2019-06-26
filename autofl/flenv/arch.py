from pprint import pformat
from typing import List

import tensorflow as tf
from tensorflow.keras.layers import (
    Activation,
    AveragePooling2D,
    BatchNormalization,
    Conv2D,
    Dense,
    Flatten,
    Input,
    MaxPool2D,
)


class Architecture:
    def __init__(self):
        self.arch: List[List[int]] = []

    def __repr__(self) -> str:
        return pformat(self.arch, indent=2)

    def get_num_layers(self) -> int:
        return len(self.arch)

    def add_layer(self, layer: List[int]) -> None:
        assert len(self.arch) == len(layer[1:])
        self.arch.append(layer)

    def get_layer(self, index: int) -> List[int]:
        assert index < len(self.arch)
        return self.arch[index]


def parse_arch_str(arch_strs: List[str]) -> Architecture:
    arch_ints: List[int] = list(map(int, arch_strs))
    arch = Architecture()
    take = 1
    while len(arch_ints) >= take:
        next_layer = arch_ints[0:take]
        arch.add_layer(next_layer)
        arch_ints = arch_ints[take:]
        take += 1
        if arch_ints:
            assert not len(arch_ints) < take
    return arch


def build_architecture(
    arch: Architecture, input_shape=(32, 32, 3), num_classes=10
) -> tf.keras.Model:
    inputs = Input(shape=input_shape)
    x = inputs
    out_filters = 24
    for layer_index in range(arch.get_num_layers()):
        layer = arch.get_layer(layer_index)
        op_index = layer[0]
        _scs = layer[1:]  # Skip connections
        op_fn = op_fns[op_index]
        x = op_fn(x, out_filters)
        if op_index in [4, 5]:
            out_filters *= 2
    # Softmax
    x = Flatten()(x)
    outputs = Dense(num_classes, activation="softmax")(x)
    return tf.keras.Model(inputs=inputs, outputs=outputs)


def apply_conv_3x3(x, out_filters) -> tf.Tensor:
    x = Activation("relu")(x)
    x = Conv2D(filters=out_filters, kernel_size=3, padding="same")(x)
    x = BatchNormalization()(x)
    return x


def apply_conv_5x5(x, out_filters) -> tf.Tensor:
    x = Activation("relu")(x)
    x = Conv2D(filters=out_filters, kernel_size=5, padding="same")(x)
    x = BatchNormalization()(x)
    return x


def apply_ds_conv_3x3(x, out_filters) -> tf.Tensor:
    x = Activation("relu")(x)
    x = Conv2D(filters=out_filters, kernel_size=3, padding="same")(x)
    x = BatchNormalization()(x)
    return x


def apply_ds_conv_5x5(x, out_filters) -> tf.Tensor:
    x = Activation("relu")(x)
    x = Conv2D(filters=out_filters, kernel_size=5, padding="same")(x)
    x = BatchNormalization()(x)
    return x


def apply_avg_pooling(x, _out_filters) -> tf.Tensor:
    return AveragePooling2D(pool_size=2)(x)


def apply_max_pooling(x, _out_filters) -> tf.Tensor:
    return MaxPool2D(pool_size=2)(x)


op_fns = {
    0: apply_conv_3x3,
    1: apply_conv_5x5,
    2: apply_ds_conv_3x3,
    3: apply_ds_conv_5x5,
    4: apply_avg_pooling,
    5: apply_max_pooling,
}
