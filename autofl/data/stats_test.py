import numpy as np
import pytest

from autofl.data.stats import DatasetStats, basic_stats, basic_stats_multiple


@pytest.mark.xfail(strict=True)
def test_create_dataset_stats_failure():
    DatasetStats()


def test_create_dataset_stats():
    number_of_examples = 1
    number_of_examples_per_label = (np.ndarray((1)), np.ndarray((1)))

    stats = DatasetStats(
        number_of_examples=number_of_examples,
        number_of_examples_per_label=number_of_examples_per_label,
    )

    assert stats["number_of_examples"] == 1
    assert stats["number_of_examples_per_label"] == number_of_examples_per_label


def test_basic_stats(dataset):
    (x, y, _, _) = dataset
    stats = basic_stats((x, y))

    assert isinstance(stats, DatasetStats)
    assert "number_of_examples" in stats
    assert "number_of_examples_per_label" in stats

    assert stats["number_of_examples"] == 60
    assert len(stats["number_of_examples_per_label"][0]) == 10

    for count in stats["number_of_examples_per_label"][1]:
        assert count == 6


def test_basic_stats_multiple(dataset):
    (x1, y1, x2, y2) = dataset
    stats_list = basic_stats_multiple([(x1, y1), (x2, y2)])

    for stats in stats_list:
        assert isinstance(stats, DatasetStats)
        assert "number_of_examples" in stats
        assert "number_of_examples_per_label" in stats
