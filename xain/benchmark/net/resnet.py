# Derived from: https://github.com/keras-team/keras/blob/master/examples/cifar10_resnet.py
# pylint: skip-file

import math
from typing import Optional, Tuple

import tensorflow as tf
import tensorflow.keras as keras
from tensorflow.keras.layers import (
    Activation,
    AveragePooling2D,
    BatchNormalization,
    Conv2D,
    Dense,
    Dropout,
    Flatten,
    Input,
)
from tensorflow.keras.models import Model
from tensorflow.keras.regularizers import l2

L2_DEFAULT: float = 1e-4
KERNEL_INITIALIZER_DEFAULT = "he_normal"


def resnet20v2_compiled(
    input_shape=(32, 32, 3),  # CIFAR
    num_classes=10,
    lr_initial: float = 0.1,
    momentum: float = 0.9,
    k: float = 0.15,
) -> tf.keras.Model:
    model, _ = resnet(input_shape=(32, 32, 3), num_classes=10, version=2, n=2)

    def exp_decay(epoch: int) -> float:
        return lr_initial * math.exp(-k * epoch)

    optimizer = tf.keras.optimizers.SGD(lr=exp_decay(0), momentum=momentum)
    model.compile(
        optimizer=optimizer, loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model


def resnet(
    input_shape: Tuple[int, int, int] = (32, 32, 3),
    num_classes: int = 10,
    version: int = 1,
    n: int = 3,
    kernel_initializer: str = KERNEL_INITIALIZER_DEFAULT,
    l2_factor: float = L2_DEFAULT,
    l2_dense: Optional[float] = None,
    dropout: Optional[float] = None,
):
    # Computed depth from supplied model parameter n
    if version == 1:
        depth = n * 6 + 2
    elif version == 2:
        depth = n * 9 + 2

    # Model name, depth and version
    model_type = "ResNet%dv%d" % (depth, version)

    # Build model
    if version == 2:
        model = resnet_v2(
            input_shape=input_shape,
            depth=depth,
            l2_factor=l2_factor,
            l2_dense=l2_dense,
            dropout=dropout,
        )
    else:
        model = resnet_v1(
            input_shape=input_shape,
            depth=depth,
            l2_factor=l2_factor,
            l2_dense=l2_dense,
            dropout=dropout,
        )
    return model, model_type


def resnet_layer(
    inputs,
    num_filters=16,
    kernel_size=3,
    strides=1,
    activation="relu",
    batch_normalization=True,
    conv_first=True,
    kernel_initializer: str = KERNEL_INITIALIZER_DEFAULT,
    l2_factor=L2_DEFAULT,
    dropout: Optional[float] = None,
):
    """2D Convolution-Batch Normalization-Activation stack builder

    # Arguments
        inputs (tensor): input tensor from input image or previous layer
        num_filters (int): Conv2D number of filters
        kernel_size (int): Conv2D square kernel dimensions
        strides (int): Conv2D square stride dimensions
        activation (string): activation name
        batch_normalization (bool): whether to include batch normalization
        conv_first (bool): conv-bn-activation (True) or
            bn-activation-conv (False)

    # Returns
        x (tensor): tensor as input to the next layer
    """
    conv = Conv2D(
        num_filters,
        kernel_size=kernel_size,
        strides=strides,
        padding="same",
        kernel_initializer=kernel_initializer,
        kernel_regularizer=l2(l2_factor),
    )

    x = inputs
    if conv_first:
        x = conv(x)
        if batch_normalization:
            x = BatchNormalization()(x)
        if activation is not None:
            x = Activation(activation)(x)
    else:
        if batch_normalization:
            x = BatchNormalization()(x)
        if activation is not None:
            x = Activation(activation)(x)
        x = conv(x)
    if dropout is not None and dropout > 0.0:
        x = Dropout(dropout)(x)
    return x


def resnet_v1(
    input_shape,
    depth,
    num_classes=10,
    kernel_initializer: str = KERNEL_INITIALIZER_DEFAULT,
    l2_factor=L2_DEFAULT,
    l2_dense: Optional[float] = None,
    dropout: Optional[float] = None,
):
    """ResNet Version 1 Model builder [a]

    Stacks of 2 x (3 x 3) Conv2D-BN-ReLU
    Last ReLU is after the shortcut connection.
    At the beginning of each stage, the feature map size is halved (downsampled)
    by a convolutional layer with strides=2, while the number of filters is
    doubled. Within each stage, the layers have the same number filters and the
    same number of filters.
    Features maps sizes:
    stage 0: 32x32, 16
    stage 1: 16x16, 32
    stage 2:  8x8,  64
    The Number of parameters is approx the same as Table 6 of [a]:
    ResNet20 0.27M
    ResNet32 0.46M
    ResNet44 0.66M
    ResNet56 0.85M
    ResNet110 1.7M

    # Arguments
        input_shape (tensor): shape of input image tensor
        depth (int): number of core convolutional layers
        num_classes (int): number of classes (CIFAR10 has 10)

    # Returns
        model (Model): Keras model instance
    """
    if (depth - 2) % 6 != 0:
        raise ValueError("depth should be 6n+2 (eg 20, 32, 44 in [a])")
    # Start model definition.
    num_filters = 16
    num_res_blocks = int((depth - 2) / 6)

    inputs = Input(shape=input_shape)
    x = resnet_layer(inputs=inputs, kernel_initializer=kernel_initializer)
    # Instantiate the stack of residual units
    for stack in range(3):
        for res_block in range(num_res_blocks):
            strides = 1
            if stack > 0 and res_block == 0:  # first layer but not first stack
                strides = 2  # downsample
            y = resnet_layer(
                inputs=x,
                num_filters=num_filters,
                strides=strides,
                kernel_initializer=kernel_initializer,
                l2_factor=l2_factor,
                dropout=dropout,
            )
            y = resnet_layer(
                inputs=y,
                num_filters=num_filters,
                activation=None,
                kernel_initializer=kernel_initializer,
                l2_factor=l2_factor,
                dropout=dropout,
            )
            if stack > 0 and res_block == 0:  # first layer but not first stack
                # linear projection residual shortcut connection to match
                # changed dims
                x = resnet_layer(
                    inputs=x,
                    num_filters=num_filters,
                    kernel_size=1,
                    strides=strides,
                    activation=None,
                    batch_normalization=False,
                    kernel_initializer=kernel_initializer,
                    l2_factor=l2_factor,
                    dropout=dropout,
                )
            x = keras.layers.add([x, y])
            x = Activation("relu")(x)
        num_filters *= 2

    # Add classifier on top.
    # v1 does not use BN after last shortcut connection-ReLU
    x = AveragePooling2D(pool_size=8)(x)
    y = Flatten()(x)
    outputs = Dense(
        num_classes,
        activation="softmax",
        kernel_initializer="he_normal",
        kernel_regularizer=l2(l2_dense) if l2_dense is not None else None,
    )(y)

    # Instantiate model.
    model = Model(inputs=inputs, outputs=outputs)
    return model


def resnet_v2(
    input_shape,
    depth,
    num_classes=10,
    kernel_initializer: str = KERNEL_INITIALIZER_DEFAULT,
    l2_factor=L2_DEFAULT,
    l2_dense: Optional[float] = None,
    dropout: Optional[float] = None,
):
    """ResNet Version 2 Model builder [b]

    Stacks of (1 x 1)-(3 x 3)-(1 x 1) BN-ReLU-Conv2D or also known as
    bottleneck layer
    First shortcut connection per layer is 1 x 1 Conv2D.
    Second and onwards shortcut connection is identity.
    At the beginning of each stage, the feature map size is halved (downsampled)
    by a convolutional layer with strides=2, while the number of filter maps is
    doubled. Within each stage, the layers have the same number filters and the
    same filter map sizes.
    Features maps sizes:
    conv1  : 32x32,  16
    stage 0: 32x32,  64
    stage 1: 16x16, 128
    stage 2:  8x8,  256

    # Arguments
        input_shape (tensor): shape of input image tensor
        depth (int): number of core convolutional layers
        num_classes (int): number of classes (CIFAR10 has 10)

    # Returns
        model (Model): Keras model instance
    """
    if (depth - 2) % 9 != 0:
        raise ValueError("depth should be 9n+2 (eg 56 or 110 in [b])")
    # Start model definition.
    num_filters_in = 16
    num_res_blocks = int((depth - 2) / 9)

    inputs = Input(shape=input_shape)
    # v2 performs Conv2D with BN-ReLU on input before splitting into 2 paths
    x = resnet_layer(
        inputs=inputs,
        num_filters=num_filters_in,
        conv_first=True,
        kernel_initializer=kernel_initializer,
    )

    # Instantiate the stack of residual units
    for stage in range(3):
        for res_block in range(num_res_blocks):
            activation: Optional[str] = "relu"
            batch_normalization = True
            strides = 1
            if stage == 0:
                num_filters_out = num_filters_in * 4
                if res_block == 0:  # first layer and first stage
                    activation = None
                    batch_normalization = False
            else:
                num_filters_out = num_filters_in * 2
                if res_block == 0:  # first layer but not first stage
                    strides = 2  # downsample

            # bottleneck residual unit
            y = resnet_layer(
                inputs=x,
                num_filters=num_filters_in,
                kernel_size=1,
                strides=strides,
                activation=activation,
                batch_normalization=batch_normalization,
                conv_first=False,
                kernel_initializer=kernel_initializer,
                l2_factor=l2_factor,
                dropout=dropout,
            )
            y = resnet_layer(
                inputs=y,
                num_filters=num_filters_in,
                conv_first=False,
                kernel_initializer=kernel_initializer,
                l2_factor=l2_factor,
                dropout=dropout,
            )
            y = resnet_layer(
                inputs=y,
                num_filters=num_filters_out,
                kernel_size=1,
                conv_first=False,
                kernel_initializer=kernel_initializer,
                l2_factor=l2_factor,
                dropout=dropout,
            )
            if res_block == 0:
                # linear projection residual shortcut connection to match
                # changed dims
                x = resnet_layer(
                    inputs=x,
                    num_filters=num_filters_out,
                    kernel_size=1,
                    strides=strides,
                    activation=None,
                    batch_normalization=False,
                    kernel_initializer=kernel_initializer,
                    l2_factor=l2_factor,
                    dropout=dropout,
                )
            x = keras.layers.add([x, y])

        num_filters_in = num_filters_out

    # Add classifier on top.
    # v2 has BN-ReLU before Pooling
    x = BatchNormalization()(x)
    x = Activation("relu")(x)
    x = AveragePooling2D(pool_size=8)(x)
    y = Flatten()(x)
    outputs = Dense(
        num_classes,
        activation="softmax",
        kernel_initializer=kernel_initializer,
        kernel_regularizer=l2(l2_dense) if l2_dense is not None else None,
    )(y)

    # Instantiate model.
    model = Model(inputs=inputs, outputs=outputs)
    return model
