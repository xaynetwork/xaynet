from .stats import DSStats


def test_DSStats_all(mock_federated_dataset):
    ds_stats = DSStats("fed_ds", mock_federated_dataset)

    stats = ds_stats.all()

    assert isinstance(stats, dict)
    assert isinstance(stats["number_of_examples_per_label_per_shard"], dict)

    for key, stat in stats["number_of_examples_per_label_per_shard"].items():
        assert "total" in stat
        assert "per_label" in stat

        if key == "val":
            assert stat["total"] == 60
        elif key == "test":
            assert stat["total"] == 100
        else:
            assert stat["total"] == 270

        assert isinstance(stat["per_label"], list)
        assert len(stat["per_label"]) == 10
