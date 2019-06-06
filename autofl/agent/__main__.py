import tensorflow as tf

from autofl.agent import agent

# See: https://stackoverflow.com/questions/33703624/how-does-tf-app-run-work
tf.app.run(main=agent.main)
