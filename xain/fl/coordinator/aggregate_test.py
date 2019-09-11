import numpy as np

from . import aggregate


def test_federated_averaging():  # pylint: disable=too-many-locals
    # Prepare:
    # - Three weight updates (u0, u1, u2)
    # - One layer in the model
    # - Two weight tensors in the layer (e.g. weights + bias)

    u0_l1_w0 = np.array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    u0_l1_w1 = np.ones((2))
    u0 = [u0_l1_w0, u0_l1_w1]

    u1_l1_w0 = np.array([[2.0, 3.0, 1.0], [4.0, 5.0, 6.0]])
    u1_l1_w1 = np.ones((2))
    u1 = [u1_l1_w0, u1_l1_w1]

    u2_l1_w0 = np.array([[3.0, 1.0, 2.0], [4.0, 5.0, 6.0]])
    u2_l1_w1 = np.ones((2))
    u2 = [u2_l1_w0, u2_l1_w1]

    thetas = [u0, u1, u2]

    theta_expected = [
        np.array([[2.0, 2.0, 2.0], [4.0, 5.0, 6.0]]),
        np.array([1.0, 1.0]),
    ]

    weighting = np.ones((len(thetas)))

    # Execute
    theta_actual = aggregate.federated_averaging(thetas, weighting)

    # Assert
    assert len(theta_actual) == len(theta_expected)

    for w_index, w_actual in enumerate(theta_actual):
        w_expected = theta_expected[w_index]
        np.testing.assert_array_equal(w_actual, w_expected)
