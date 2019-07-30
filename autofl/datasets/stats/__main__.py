from absl import app, logging

from autofl.datasets import (
    cifar10_random_splits_10,
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
    fashion_mnist_100p_IID_balanced,
)

from .stats import DSStats


def main(_):
    logging.info(
        DSStats(
            name="cifar10_random_splits_10", ds=cifar10_random_splits_10.load_splits()
        )
    )

    logging.info(
        DSStats(
            name="fashion_mnist_10s_500_1k_bias",
            ds=fashion_mnist_10s_500_1k_bias.load_splits(),
        )
    )

    logging.info(
        DSStats(name="fashion_mnist_10s_600", ds=fashion_mnist_10s_600.load_splits())
    )

    logging.info(
        DSStats(
            name="fashion_mnist_10s_single_class",
            ds=fashion_mnist_10s_single_class.load_splits(),
        )
    )

    logging.info(
        DSStats(
            name="fashion_mnist_100p_IID_balanced",
            ds=fashion_mnist_100p_IID_balanced.load_splits(),
        )
    )


app.run(main=main)
