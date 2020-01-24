"""XAIN FL tests for coordinator aggregation"""

# TODO: (XP-351) decide how to continue with that
# import numpy as np
#
# from xain_fl.fl.coordinator.aggregate import federated_averaging
#
#
# def test_federated_averaging():  # pylint: disable=too-many-locals
#     """[summary]
#
#     .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)
#     """
#
#     # Prepare:
#     # - Three weight updates (u0, u1, u2)
#     # - One layer in the model
#     # - Two weight tensors in the layer (e.g. weights + bias)
#
#     u0_l1_w0 = np.array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
#     u0_l1_w1 = np.ones((2))
#     u0_ = [u0_l1_w0, u0_l1_w1]
#
#     u1_l1_w0 = np.array([[2.0, 3.0, 1.0], [4.0, 5.0, 6.0]])
#     u1_l1_w1 = np.ones((2))
#     u1_ = [u1_l1_w0, u1_l1_w1]
#
#     u2_l1_w0 = np.array([[3.0, 1.0, 2.0], [4.0, 5.0, 6.0]])
#     u2_l1_w1 = np.ones((2))
#     u2_ = [u2_l1_w0, u2_l1_w1]
#
#     model_weights = [u0_, u1_, u2_]
#
#     model_weights_expected = [
#         np.array([[2.0, 2.0, 2.0], [4.0, 5.0, 6.0]]),
#         np.array([1.0, 1.0]),
#     ]
#
#     weighting = np.ones((len(model_weights)))
#
#     # Execute
#     model_weights_actual = federated_averaging(model_weights, weighting)
#
#     # Assert
#     assert len(model_weights_actual) == len(model_weights_expected)
#
#     for w_index, w_actual in enumerate(model_weights_actual):
#         w_expected = model_weights_expected[w_index]
#         np.testing.assert_array_equal(w_actual, w_expected)
