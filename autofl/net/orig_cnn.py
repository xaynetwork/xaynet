import tensorflow as tf
from tensorflow.keras.layers import Conv2D, Dense, Flatten, Input, MaxPool2D


def orig_cnn_compiled(input_shape=(28, 28, 1), num_classes=10) -> tf.keras.Model:
    inputs = Input(shape=input_shape)
    x = Conv2D(32, kernel_size=5, activation="relu")(inputs)
    x = MaxPool2D(pool_size=2)(x)
    x = Conv2D(64, kernel_size=5, activation="relu")(x)
    x = MaxPool2D(pool_size=2)(x)
    x = Flatten()(x)
    x = Dense(512, activation="relu")(x)
    outputs = Dense(num_classes, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )
    return model
