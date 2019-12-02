import numpy as np
import tensorflow as tf
from tensorflow.data import Dataset

from xain_fl.types import Partition

AUTOTUNE = tf.data.experimental.AUTOTUNE
SEED = 2017


def init_ds_train(
    xy: Partition, num_classes=10, batch_size=32, augmentation=False
) -> Dataset:
    """Initializes federated train dataset partition. Will return a TensorFlow dataset.

    Args:
        xy (Partition): Tuple of two ndarrays corresponding to (examples, classes)
        num_classes (int): Number of classes present in parition
        batch_size (int): Number of examples in each batch
        augmentation (bool): Enables automated augmentation of dataset e.g. random
            cropping or random hue, saturation, brightness or contrast adjustment

    Returns:
        Dataset: TensorFlow dataset
    """
    return _init_ds(xy, num_classes, batch_size, augmentation, shuffle=True)


def init_ds_val(xy: Partition, num_classes=10) -> Dataset:
    """Initializes validation partition. Will return a TensorFlow dataset.

    Args:
        xy (Partition): Tuple of two ndarrays corresponding to (examples, classes)
        num_classes (int): Number of classes present in federated dataset partition

    Returns:
        Dataset: TensorFlow dataset
    """
    batch_size = xy[0].shape[0]  # Return full dataset as one large batch
    return _init_ds(xy, num_classes, batch_size, augmentation=False, shuffle=False)


# pylint: disable-msg=too-many-arguments
def _init_ds(
    xy: Partition, num_classes: int, batch_size: int, augmentation: bool, shuffle: bool
) -> Dataset:
    (x, y) = xy
    # Assume that each row in `x` corresponds to the same row in `y`
    assert x.shape[0] == y.shape[0]
    assert x.ndim == 3 or x.ndim == 4  # (Fashion-)MNIST: 3, CIFAR-10: 4
    assert y.ndim == 1
    # Add one dimension to grayscale-image datasets
    grayscale = False
    if x.ndim == 3:
        grayscale = True
        x = np.reshape(x, (x.shape[0], x.shape[1], x.shape[2], 1))
    # Create tf.data.Dataset from ndarrays
    ds = to_dataset(x, y)
    # Data preparation:
    # - Cast color channel values to float, divide by 255
    # - One-hot encode labels
    ds = prepare(ds, num_classes=num_classes)
    # Data augmentation:
    # - Randomize hue/saturation/brightness/contrast (CIFAR-10/non-grayscale only)
    # - Take random 32x32 (or 28x28) crop (after padding to 40x40 (or 32x32))
    # - Random horizontal flip
    if augmentation:
        ds = _augment_ds(ds, grayscale)
    return batch_and_repeat(ds, batch_size, shuffle=shuffle, repeat=True)


def to_dataset(x: np.ndarray, y: np.ndarray) -> Dataset:
    """Creates a TensorFlow Dataset from two ndarrays

    Args:
        x (np.ndarray)
        y (np.ndarray)

    Returns:
        Dataset
    """
    return Dataset.from_tensor_slices((x, y))


def prepare(ds: Dataset, num_classes: int) -> Dataset:
    """Prepares dataset for training by
    - Casting color channel values to float, divide by 255
    - One-hot encode labels

    Args:
        ds (Dataset): TensorFlow Dataset
        num_classes (int): Number of classes present in federated dataset partition

    Returns:
        Dataset
    """
    ds = ds.map(lambda x, y: (x, _prep_cast_label(y)))
    ds = ds.map(lambda x, y: (_prep_cast_divide(x), y))
    ds = ds.map(lambda x, y: (x, _prep_one_hot(y, num_classes)))
    return ds


def _augment_ds(ds: Dataset, grayscale: bool) -> Dataset:
    if not grayscale:
        ds = ds.map(
            lambda x, y: (_random_hue_saturation_brightness_contrast(x), y),
            num_parallel_calls=AUTOTUNE,
        )
    if grayscale:
        ds = ds.map(
            lambda x, y: (_random_crop_mnist(x), y), num_parallel_calls=AUTOTUNE
        )
    else:
        ds = ds.map(
            lambda x, y: (_random_crop_cifar(x), y), num_parallel_calls=AUTOTUNE
        )
    ds = ds.map(
        lambda x, y: (_random_horizontal_flip(x), y), num_parallel_calls=AUTOTUNE
    )
    return ds


def batch_and_repeat(
    ds: Dataset, batch_size: int, shuffle: bool, repeat: bool
) -> Dataset:
    """Helper method for to apply tensorflow shuffle, repeat and
    batch (in this order)

    Args:
        ds (Dataset): Tensorflow Dataset
        batch_size (int): Will call ds.batch(batch_size, drop_remainder=False)
            if batch_size is greater zero
        shuffle (int): Will call ds.shuffle(1024)
        repeat (bool): Will call ds.repeat()

    Returns:
        Dataset
    """
    ds = ds.prefetch(buffer_size=AUTOTUNE)
    if shuffle:
        ds = ds.shuffle(1024, seed=SEED)
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


def _random_crop_mnist(img: tf.Tensor) -> tf.Tensor:
    img_padded = tf.image.pad_to_bounding_box(img, 2, 2, 32, 32)
    return tf.image.random_crop(img_padded, size=[28, 28, 1], seed=SEED)


def _random_crop_cifar(img: tf.Tensor) -> tf.Tensor:
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
