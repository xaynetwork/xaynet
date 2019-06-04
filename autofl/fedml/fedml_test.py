import numpy as np
from autofl.fedml import fedml


def test_Participant_x_y_shape_valid():
    # Prepare
    m = None
    x = np.zeros((3, 28, 28))
    y = np.zeros((3))
    # Execute
    _ = fedml.Participant(m, x, y)
    # Assert
    pass


def test_Participant_x_y_shape_invalid():
    # Prepare
    m = None
    x = np.zeros((3, 28, 28))
    y = np.zeros((4))
    # Execute & assert
    try:
        _ = fedml.Participant(m, x, y)
        fail()
    except:
        pass
