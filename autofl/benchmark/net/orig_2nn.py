import tensorflow as tf
from tensorflow.keras.layers import Dense, Flatten, Input


def orig_2nn_compiled(
    input_shape=(28, 28, 1), num_classes=10, seed: int = 2017
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

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.SGD(),
        metrics=["accuracy"],
    )
    return model
