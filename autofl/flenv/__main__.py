import gym
import numpy as np
from absl import app, logging

from . import flenv


def main(_):
    # Init env
    flenv.register_gym_env()
    env = gym.make("FederatedLearning-v0")
    # Exercise for a few steps
    s = env.reset()
    for step in range(env.num_rounds):
        a = np.random.random_integers(0, 1, env.num_participants)
        s_prime, r, d, _ = env.step(a)
        exp = (s, a, r, s_prime, d)
        logging.info(
            "\n\nRound {}, experience:\ns:\n{}\na: {} \nr: {}\ns':\n{}\nd: {}\n\n".format(
                step, exp[0], exp[1], exp[2], exp[3], exp[4]
            )
        )
        if step < env.num_rounds - 1:
            assert not d
        else:
            assert d
        s = s_prime


app.run(main=main)
