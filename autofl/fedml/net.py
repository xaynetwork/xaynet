import tensorflow as tf

from ..flenv import arch


def fc() -> tf.keras.Model:
    model = tf.keras.models.Sequential(
        [
            tf.keras.layers.Flatten(input_shape=(28, 28)),
            tf.keras.layers.Dense(128, activation="relu"),
            tf.keras.layers.Dense(10, activation="softmax"),
        ]
    )
    model.compile(
        optimizer="adam", loss="sparse_categorical_crossentropy", metrics=["accuracy"]
    )
    return model


def cnn() -> tf.keras.Model:
    arch_str = [str(x) for x in [1, 2, 0, 3, 0, 0]]
    model = arch.build_architecture(arch.parse_arch_str(arch_str))
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model
