from autofl import main


def test_integer_addition():
  expected = 3
  actual = main.integer_addition(1, 2)
  assert expected == actual
