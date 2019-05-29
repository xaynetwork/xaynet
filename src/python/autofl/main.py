import tensorflow as tf
import mnist_f


def main():
  x_splits, y_splits, x_test, y_test = mnist_f.load_splits()
  print(len(x_splits))
  print(len(y_splits))


def tf_hello_world():
  # Source: https://www.tensorflow.org/overview/
  mnist = tf.keras.datasets.mnist

  (x_train, y_train),(x_test, y_test) = mnist.load_data()
  x_train, x_test = x_train / 255.0, x_test / 255.0

  model = tf.keras.models.Sequential([
    tf.keras.layers.Flatten(input_shape=(28, 28)),
    tf.keras.layers.Dense(128, activation='relu'),
    tf.keras.layers.Dropout(0.2),
    tf.keras.layers.Dense(10, activation='softmax')
  ])

  model.compile(optimizer='adam',
                loss='sparse_categorical_crossentropy',
                metrics=['accuracy'])

  model.fit(x_train, y_train, epochs=5)
  model.evaluate(x_test, y_test)


def integer_addition(x: int, y: int) -> int:
  return x + y


if __name__ == "__main__":
  main()
