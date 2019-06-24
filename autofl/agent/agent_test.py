from . import agent


def test_sample_architecture():
    # Prepare
    expected = 5
    # Execute
    arch = agent.sample_architecture(num_layers=expected)
    # Assert
    assert arch.get_num_layers() == expected
