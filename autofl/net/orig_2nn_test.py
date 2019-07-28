from .orig_2nn import orig_2nn_compiled


def test_num_parameters():
    # Prepare
    model = orig_2nn_compiled()
    # Execute
    num_params = model.count_params()
    # Assert
    assert num_params == 199_210
