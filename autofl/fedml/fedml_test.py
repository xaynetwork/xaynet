from autofl.fedml import fedml


def test_integer_addition():
    expected = 3
    actual = fedml.integer_addition(1, 2)
    assert expected == actual
