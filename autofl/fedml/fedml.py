import tensorflow as tf
from autofl.mnist_f import mnist_f
from autofl.fedml import net


def main():
    # Load data
    x_splits, y_splits, x_test, y_test = mnist_f.load_splits()
    print(len(x_splits))
    print(len(y_splits))
    # Create model
    model = net.fc()
    model.summary()
    # Train model using only one partition
    x_train = x_splits[0]
    y_train = y_splits[0]
    x_train, x_test = x_train / 255.0, x_test / 255.0
    model.fit(x_train, y_train, epochs=5)
    model.evaluate(x_test, y_test)


def integer_addition(x: int, y: int) -> int:
    return x + y
