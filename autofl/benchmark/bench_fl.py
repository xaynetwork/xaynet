from absl import app, logging

from autofl.datasets import (
    fashion_mnist_10s_500_1k_bias,
    fashion_mnist_10s_600,
    fashion_mnist_10s_single_class,
    fashion_mnist_100p_IID_balanced,
)

from . import report, run

FLH_C = 0.1  # Fraction of participants used in each round of training
FLH_E = 1  # Number of training episodes in each round
FLH_B = 32  # Batch size used by participants

ROUNDS = 50


def benchmark_ul_fl_FashionMNIST_100p_IID_balanced():
    fn_name = benchmark_ul_fl_FashionMNIST_100p_IID_balanced.__name__
    logging.info("Starting {}".format(fn_name))

    xy_parts, xy_val, xy_test = fashion_mnist_100p_IID_balanced.load_splits()
    _run_unitary_versus_federated(xy_parts, xy_val, xy_test, C=0.1)


def benchmark_ul_fl_FashionMNIST_10p_IID_balanced():
    fn_name = benchmark_ul_fl_FashionMNIST_10p_IID_balanced.__name__
    logging.info("Starting {}".format(fn_name))
    xy_splits, xy_val, xy_test = fashion_mnist_10s_600.load_splits()
    _run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def benchmark_ul_fl_FashionMNIST_10p_1000():
    fn_name = benchmark_ul_fl_FashionMNIST_10p_1000.__name__
    logging.info("Starting {}".format(fn_name))
    xy_splits, xy_val, xy_test = fashion_mnist_10s_500_1k_bias.load_splits()
    _run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def benchmark_ul_fl_FashionMNIST_10p_5400():
    fn_name = benchmark_ul_fl_FashionMNIST_10p_5400.__name__
    logging.info("Starting {}".format(fn_name))
    xy_splits, xy_val, xy_test = fashion_mnist_10s_single_class.load_splits()
    _run_unitary_versus_federated(xy_splits, xy_val, xy_test, C=0.3)


def _run_unitary_versus_federated(xy_splits, xy_val, xy_test, C):
    # TODO train n models on all partitions

    # Train CNN on a single partition ("unitary learning")
    partition_id = 0
    logging.info("Run unitary training using partition {}".format(partition_id))
    ul_hist, ul_loss, ul_acc = run.unitary_training(
        xy_splits[partition_id], xy_val, xy_test, epochs=ROUNDS, batch_size=FLH_B
    )

    # Train CNN using federated learning on all partitions
    logging.info("Run federated learning using all partitions")
    fl_hist, fl_loss, fl_acc = run.federated_training(
        xy_splits, xy_val, xy_test, ROUNDS, C=C, E=FLH_E, B=FLH_B
    )

    # Output results
    report.plot_accuracies(ul_hist, fl_hist, fname="UL-FL.png")
    logging.info("UL test set loss: {}, accuracy: {}".format(ul_loss, ul_acc))
    logging.info("FL test set loss: {}, accuracy: {}".format(fl_loss, fl_acc))


def main(_):
    benchmark_ul_fl_FashionMNIST_10p_IID_balanced()
    # benchmark_ul_fl_FashionMNIST_10p_1000()
    # benchmark_ul_fl_FashionMNIST_10p_5400()
    # benchmark_ul_fl_FashionMNIST_100p_IID_balanced()


if __name__ == "__main__":
    app.run(main=main)
