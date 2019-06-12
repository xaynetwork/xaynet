from autofl.data.stats import basic_statistics


def test_basic_statistics(dataset):
    stats = basic_statistics(dataset)

    assert isinstance(stats, dict)
    assert isinstance(stats["train"], dict)
    assert isinstance(stats["test"], dict)

    assert stats["train"]["number_of_examples"] == 60
    assert stats["test"]["number_of_examples"] == 10

    assert len(stats["train"]["number_of_examples_per_label"][0]) == 10
    assert len(stats["test"]["number_of_examples_per_label"][0]) == 10

    for count in stats["train"]["number_of_examples_per_label"][1]:
        assert count == 6

    for count in stats["test"]["number_of_examples_per_label"][1]:
        assert count == 1
