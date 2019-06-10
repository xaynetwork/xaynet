import tensorflow as tf

from . import agent

tf.app.flags.DEFINE_string(
    "arch", None, "Provide a default network architecture to use"
)

# See: https://stackoverflow.com/questions/33703624/how-does-tf-app-run-work
tf.app.run(main=agent.main)
