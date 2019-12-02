from .coordinator import _abs_C


def test_abs_C_min():
    # Prepare
    num_participants = 100
    C = 0.0
    # Execute
    actual = _abs_C(C, num_participants)
    # Assert
    assert actual == 1


def test_abs_C_point_1():
    # Prepare
    num_participants = 100
    C = 0.1
    # Execute
    actual = _abs_C(C, num_participants)
    # Assert
    assert actual == 10


def test_abs_C_point_5():
    # Prepare
    num_participants = 100
    C = 0.5
    # Execute
    actual = _abs_C(C, num_participants)
    # Assert
    assert actual == 50


def test_abs_C_1():
    # Prepare
    num_participants = 100
    C = 1.0
    # Execute
    actual = _abs_C(C, num_participants)
    # Assert
    assert actual == 100


def test_abs_C_2():
    # Prepare
    num_participants = 100
    C = 2.0
    # Execute
    actual = _abs_C(C, num_participants)
    # Assert
    assert actual == 100
