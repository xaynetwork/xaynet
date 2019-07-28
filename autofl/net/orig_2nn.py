import tensorflow as tf
from tensorflow.keras.layers import Dense, Flatten, Input


def orig_2nn_compiled(input_shape=(28, 28, 1), num_classes=10) -> tf.keras.Model:
    inputs = Input(shape=input_shape)
    x = Flatten()(inputs)
    x = Dense(200, activation="relu")(x)
    x = Dense(200, activation="relu")(x)
    outputs = Dense(num_classes, activation="softmax")(x)

    model = tf.keras.Model(inputs=inputs, outputs=outputs)

    model.compile(
        loss=tf.keras.losses.categorical_crossentropy,
        optimizer=tf.keras.optimizers.Adam(),
        metrics=["accuracy"],
    )
    return model
