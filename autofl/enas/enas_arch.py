from typing import Callable

import tensorflow as tf

from autofl.fl.coordinator import Coordinator

from . import arch


def enas_cnn_compiled() -> tf.keras.Model:
    arch_str = [str(x) for x in [1, 2, 0, 3, 0, 0]]
    model = arch.build_architecture(arch.parse_arch_str(arch_str))
    model.compile(
        optimizer="adam", loss="categorical_crossentropy", metrics=["accuracy"]
    )
    return model


# pylint: disable-msg=unused-variable
def replace_model(
    coordinator: Coordinator, model_fn: Callable[..., tf.keras.Model]
) -> None:
    coordinator.model = model_fn()
    for p in coordinator.participants:
        model = model_fn()
        # p.replace_model(model)
