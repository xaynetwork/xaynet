from autofl.data.stats import basic_stats, basic_stats_multiple


def test_basic_stats(dataset):
    (x, y, _, _) = dataset
    stats = basic_stats((x, y))

    assert "number_of_examples" in stats
    assert "number_of_examples_per_label" in stats

    assert stats["number_of_examples"] == 60
    assert len(stats["number_of_examples_per_label"][0]) == 10

    for count in stats["number_of_examples_per_label"][1]:
        assert count == 6


def test_basic_stats_multiple(dataset):
    (x1, y1, x2, y2) = dataset
    stats_list = basic_stats_multiple([(x1, y1), (x2, y2)])

    for stat in stats_list:
        assert "number_of_examples" in stat
        assert "number_of_examples_per_label" in stat
