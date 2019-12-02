from xain_fl.datasets.hashes import load


def test_load_hashes():
    # Execute
    dataset_hashes = load.load_hashes()

    # Prepare
    assert isinstance(dataset_hashes, dict)
    assert "fashion-mnist-100p-noniid-01cpp" in dataset_hashes.keys()
    assert len(dataset_hashes["fashion-mnist-100p-noniid-01cpp"].keys()) == 102
