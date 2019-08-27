from absl import app, logging

from .orig_2nn import orig_2nn_compiled
from .orig_cnn import orig_cnn_compiled
from .resnet import resnet20v2_compiled


def main(_):
    logging.info("\nOrig. 2NN:")
    model = orig_2nn_compiled()
    model.summary(print_fn=logging.info)

    logging.info("\nOrig. CNN (MNIST):")
    model = orig_cnn_compiled()
    model.summary(print_fn=logging.info)

    logging.info("\nOrig. CNN (CIFAR-10):")
    model = orig_cnn_compiled(input_shape=(32, 32, 3))
    model.summary(print_fn=logging.info)

    logging.info("\nResNet20v2 (CIFAR-10):")
    model = resnet20v2_compiled(input_shape=(32, 32, 3))
    model.summary(print_fn=logging.info)


if __name__ == "__main__":
    app.run(main=main)
