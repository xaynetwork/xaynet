import tensorflow as tf

from .data import load, shuffle, split
from .prep import init_dataset

# Load dataset
x_train, y_train, _, _ = load(tf.keras.datasets.cifar10)
print("Training set before split:")
print("\tx_train:", x_train.shape, type(x_train))
print("\ty_train:", y_train.shape, type(x_train))

# Shuffle both x and y with the same permutation
x_train, y_train = shuffle(x_train, y_train)
print("Training set after shuffle:")
print("\tx_train:", x_train.shape, type(x_train))
print("\ty_train:", y_train.shape, type(x_train))

x_splits, y_splits = split(x_train, y_train, num_splits=5)
print("Training set after split:")
for i, (x_split, y_split) in enumerate(zip(x_splits, y_splits)):
    print("\t", str(i), "x_split:", x_split.shape, type(x_split))
    print("\t", str(i), "y_split:", y_split.shape, type(y_split))

x, y = (x_splits[0], y_splits[0])
ds = init_dataset(x, y)
print("ds.output_types:", ds.output_types)
print("ds.output_shapes:", ds.output_shapes)
