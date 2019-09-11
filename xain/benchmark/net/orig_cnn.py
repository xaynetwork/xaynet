import tensorflow as tf
from tensorflow.keras.layers import Conv2D, Dense, Flatten, Input, MaxPool2D


# pylint: disable-msg=unused-argument
def orig_cnn_compiled(
    input_shape=(28, 28, 1),
    num_classes: int = 10,
    seed: int = 2017,
    epoch_base: int = 0,
) -> tf.keras.Model:
    # Kernel initializer
    ki = tf.keras.initializers.glorot_uniform(seed=seed)

    # Architecture
    inputs = Input(shape=input_shape)
    x = Conv2D(
        32,
        kernel_size=(5, 5),
        strides=(1, 1),
        kernel_initializer=ki,
        padding="same",
        activation="relu",
    )(inputs)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = Conv2D(
        64,
        kernel_size=(5, 5),
        strides=(1, 1),
        kernel_initializer=ki,
        padding="same",
        activation="relu",
    )(x)
    x = MaxPool2D(pool_size=(2, 2), strides=(2, 2))(x)
    x = Flatten()(x)
    x = Dense(512, kernel_initializer=ki, activation="relu")(x)
    outputs = Dense(num_classes, kernel_initializer=ki, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.SGD(),
        metrics=["accuracy"],
    )
    return model
