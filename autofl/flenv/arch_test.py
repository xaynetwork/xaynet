from . import arch as architecture


def test_parse_arch_str():
    # Prepare
    arch_strs = ["0", "1", "1"]
    # Execute
    arch = architecture.parse_arch_str(arch_strs)
    # Assert
    assert arch.get_num_layers() == 2
    assert len(arch.get_layer(0)) == 1
    assert len(arch.get_layer(1)) == 2
