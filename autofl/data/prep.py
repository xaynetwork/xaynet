import numpy as np
import tensorflow as tf
from tensorflow.data import Dataset

AUTOTUNE = tf.data.experimental.AUTOTUNE
SEED = 1096


def init_dataset(x: np.ndarray, y: np.ndarray) -> Dataset:
    # Assume that each row in `x` corresponds to the same row in `y`
    assert x.shape[0] == y.shape[0]
    assert x.ndim == 3 or x.ndim == 4  # MNIST: 3, CIFAR-10: 4
    assert y.ndim == 1
    # Create tf.data.Dataset from ndarrays
    ds = to_dataset(x, y)
    # Data preparation:
    # - Cast color channel values to float, divide by 255
    # - One-hot encode labels
    ds = prepare(ds, num_classes=10)
    # Data augmentation (CIFAR-10 only):
    # - Randomize hue/saturation/brightness/contrast
    # - Take random 32x32 crop (after padding to 40x40)
    # - Random horizontal flip
    if x.ndim == 4:
        ds = augment_cifar(ds)
    return batch_and_repeat(ds, batch_size=64)


def init_validation_dataset(x: np.ndarray, y: np.ndarray) -> Dataset:
    # Assume that each row in `x` corresponds to the same row in `y`
    assert x.shape[0] == y.shape[0]
    assert x.ndim == 3 or x.ndim == 4  # MNIST: 3, CIFAR-10: 4
    assert y.ndim == 1
    # Create tf.data.Dataset from ndarrays
    ds = to_dataset(x, y)
    # Data preparation:
    # - Cast color channel values to float, divide by 255
    # - One-hot encode labels
    ds = prepare(ds, num_classes=10)
    # No data augmentation or shuffle on the validation set
    return batch_and_repeat(ds, batch_size=x.shape[0], shuffle=False, repeat=True)


def to_dataset(x: np.ndarray, y: np.ndarray) -> Dataset:
    return Dataset.from_tensor_slices((x, y))


def prepare(ds: Dataset, num_classes: int) -> Dataset:
    ds = ds.map(lambda x, y: (x, _prep_cast_label(y)))
    ds = ds.map(lambda x, y: (_prep_cast_divide(x), y))
    ds = ds.map(lambda x, y: (x, _prep_one_hot(y, num_classes)))
    return ds


def augment_cifar(ds: Dataset) -> Dataset:
    ds = ds.map(
        lambda x, y: (_random_hue_saturation_brightness_contrast(x), y),
        num_parallel_calls=AUTOTUNE,
    )
    ds = ds.map(lambda x, y: (_random_crop(x), y), num_parallel_calls=AUTOTUNE)
    ds = ds.map(
        lambda x, y: (_random_horizontal_flip(x), y), num_parallel_calls=AUTOTUNE
    )
    return ds


def batch_and_repeat(
    ds: Dataset, batch_size: int, shuffle: bool = True, repeat: bool = True
) -> Dataset:
    ds = ds.prefetch(buffer_size=AUTOTUNE)
    if shuffle:
        ds = ds.shuffle(512, seed=SEED)
    if repeat:
        ds = ds.repeat()
    if batch_size > 0:
        ds = ds.batch(batch_size, drop_remainder=False)
    return ds


def _prep_cast_label(y: tf.Tensor) -> tf.Tensor:
    return tf.cast(y, tf.int64)


def _prep_cast_divide(x: tf.Tensor) -> tf.Tensor:
    return tf.cast(x, tf.float32) / 255


def _prep_one_hot(y: tf.Tensor, num_classes: int) -> tf.Tensor:
    return tf.one_hot(y, num_classes)


def _random_crop(img: tf.Tensor) -> tf.Tensor:
    img_padded = tf.image.pad_to_bounding_box(img, 4, 4, 40, 40)
    return tf.image.random_crop(img_padded, size=[32, 32, 3], seed=SEED)


def _random_horizontal_flip(img: tf.Tensor) -> tf.Tensor:
    return tf.image.random_flip_left_right(img, seed=SEED)


def _random_hue_saturation_brightness_contrast(img: tf.Tensor) -> tf.Tensor:
    img = tf.image.random_hue(img, 0.08, seed=SEED)
    img = tf.image.random_saturation(img, 0.6, 1.6, seed=SEED)
    img = tf.image.random_brightness(img, 0.05, seed=SEED)
    img = tf.image.random_contrast(img, 0.7, 1.3, seed=SEED)
    return img
