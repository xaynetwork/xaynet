from xain.datasets.hashes import load


def test_load_hashes():
    # Execute
    dataset_hashes = load.load_hashes()

    # Prepare
    assert isinstance(dataset_hashes, dict)
    assert "fashion_mnist_100p_01cpp" in dataset_hashes.keys()
    assert len(dataset_hashes["fashion_mnist_100p_01cpp"].keys()) == 102
