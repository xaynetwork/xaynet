import tensorflow as tf


def run():
    hello = tf.constant("Hello World :)")
    sess = tf.compat.v1.Session()
    print(sess.run(hello))


if __name__ == "__main__":
    run()
