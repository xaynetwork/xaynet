from absl import app, logging

from autofl.datasets import (
    cifar10_random_splits_10,
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
    fashion_mnist_100p_IID_balanced,
    fashion_mnist_100p_non_IID,
)

from .stats import DSStats


def main(_):
    datasets = [
        cifar10_random_splits_10,
        fashion_mnist_10s_500_1k_bias,
        fashion_mnist_10s_600,
        fashion_mnist_10s_single_class,
        fashion_mnist_100p_IID_balanced,
        fashion_mnist_100p_non_IID,
    ]

    for ds in datasets:
        logging.info(DSStats(name=ds.__name__, ds=ds.load_splits()))


app.run(main=main)
