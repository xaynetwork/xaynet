from mnist_f import main


def test_load():
  x_train, y_train, x_test, y_test = main.load()
  assert x_train.shape[0] == y_train.shape[0]
  assert x_test.shape[0] == y_test.shape[0]
  assert len(x_train.shape) == len(x_test.shape)
  assert len(y_train.shape) == len(y_test.shape)
