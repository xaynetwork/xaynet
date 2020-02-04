"""XAIN FL tests for coordinator aggregation"""

# TODO: (XP-351) decide how to continue with that
# import numpy as np
#
# from xain_fl.fl.coordinator.aggregate import federated_averaging
#
#
# def test_federated_averaging():  # pylint: disable=too-many-locals
#     """Test for `federated_averaging()`."""
#
#     # Prepare:
#     # - Three weight updates (u0, u1, u2)
#     # - One layer in the model
#     # - Two weight tensors in the layer (e.g. weights + bias)
#
#     u0_ = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 1.0, 1.0])
#     u1_ = np.array([2.0, 3.0, 1.0, 4.0, 5.0, 6.0, 1.0, 1.0])
#     u2_ = np.array([3.0, 1.0, 2.0, 4.0, 5.0, 6.0, 1.0, 1.0])
#     multiple_model_weights = [u0_, u1_, u2_]
#
#     model_weights_expected = np.array([2.0, 2.0, 2.0, 4.0, 5.0, 6.0, 1.0, 1.0])
#
#     weighting = np.ones((len(multiple_model_weights)))
#
#     # Execute
#     model_weights_actual = federated_averaging(multiple_model_weights, weighting)
#
#     # Assert
#     assert len(model_weights_actual) == len(model_weights_expected)
#     np.testing.assert_array_equal(model_weights_actual, model_weights_expected)
