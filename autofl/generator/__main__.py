from absl import app

from .datasets import (
    generate_cifar10_random_splits_10,
    generate_fashion_mnist_10s_500_1k_bias,
    generate_fashion_mnist_10s_600,
    generate_fashion_mnist_10s_single_class,
    generate_fashion_mnist_100p_IID_balanced,
    generate_fashion_mnist_100p_non_IID,
)


def main(_):
    # Just uncomment the methods you want to run
    generate_cifar10_random_splits_10()
    generate_fashion_mnist_10s_600()
    generate_fashion_mnist_10s_single_class()
    generate_fashion_mnist_10s_500_1k_bias()
    generate_fashion_mnist_100p_IID_balanced()
    generate_fashion_mnist_100p_non_IID()


app.run(main=main)
