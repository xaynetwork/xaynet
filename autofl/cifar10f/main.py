import os
from typing import Any, Tuple

import tensorflow as tf
import tensorflow_datasets as tfds
from tensorflow.data import Dataset

os.environ["TF_CPP_MIN_LOG_LEVEL"] = "3"
tf.logging.set_verbosity(tf.logging.ERROR)


def main() -> None:
    print("Attempting to download CIFAR-10")
    ds_train_full, ds_test, info = init_dataset()
    print("Download complete")
    log_info(ds_train_full)
    log_info(ds_test)


def init_dataset(data_dir=None) -> Tuple[Dataset, Dataset, Any]:
    (ds_train_full, ds_test), info = tfds.load(
        name="cifar10",
        split=["train", "test"],
        data_dir=data_dir,
        as_supervised=True,
        with_info=True,
    )
    input_shape: Tuple[int, int, int] = info.features["image"].shape
    num_classes: int = info.features["label"].num_classes
    m_train_full: int = info.splits["train"].num_examples
    m_test: int = info.splits["test"].num_examples
    info = (input_shape, num_classes, m_train_full, m_test)
    return ds_train_full, ds_test, info


def log_info(ds: Dataset) -> None:
    print("-" * 3, "Dataset Info", "-" * 63)
    print(ds)
    print("ds.output_shapes:\t", ds.output_shapes)
    print("ds.output_types:\t", ds.output_types)
    print("-" * 80)


if __name__ == "__main__":
    main()
