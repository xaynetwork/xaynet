import tensorflow as tf

from . import agent

tf.app.flags.DEFINE_spaceseplist(
    "arch",
    None,
    "Space-separated list of integers defining the network architecture to use",
)
tf.app.flags.DEFINE_float(
    "controller_lr", None, "Learning rate for the controller network"
)


# See: https://stackoverflow.com/questions/33703624/how-does-tf-app-run-work
tf.app.run(main=agent.main)
