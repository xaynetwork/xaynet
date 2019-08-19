import tensorflow as tf
from tensorflow.keras.layers import Conv2D, Dense, Dropout, Flatten, MaxPool2D

# Derived from:
# https://medium.com/tensorflow/hello-deep-learning-fashion-mnist-with-keras-50fcff8cd74a


def blog_cnn_compiled(
    input_shape=(28, 28, 1), num_classes: int = 10, seed: int = 1096
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

    # Compile model
    model.compile(
        loss="categorical_crossentropy", optimizer="adam", metrics=["accuracy"]
    )
    return model
