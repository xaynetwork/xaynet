from autofl.agent import agent


def test_parse_arch_str():
    # Prepare
    arch_str = "0 1 1"
    # Execute
    arch = agent.parse_arch_str(arch_str)
    # Assert
    assert arch.get_num_layers() == 2
    assert len(arch.get_layer(0)) == 1
    assert len(arch.get_layer(1)) == 2
