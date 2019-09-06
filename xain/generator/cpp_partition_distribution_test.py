import numpy as np
import pytest

from .cpp_partition_distribution import cpp_partition_distribution

# def test_Bucket(replace, num_classes, num_per_class, num_per_pick):
#     # Prepare
#     bucket = data.Bucket(num_classes=num_classes, num_per_class=num_per_class)
#     expected_bucket_size = num_classes * num_per_class
#     expected_total_picks_till_empty = math.ceil(expected_bucket_size / num_per_pick)

#     # Execute
#     if not replace and num_per_pick > num_classes:
#         with pytest.raises(Exception):
#             bucket.pick(num_per_pick, replace=replace)
#             # FIXME: don't rturn rather use some pytest method here
#         return

#     picks = np.array([], dtype=np.int8)

#     # try to pick too much
#     for _ in range(expected_total_picks_till_empty):
#         picks = np.concatenate([picks, bucket.pick(num_per_pick, replace=replace)])

#     # Assert
#     # picks should contain num_classes * num_per_class entries
#     assert picks.size == expected_bucket_size

#     # Bucket should be empty after num_per_class picks
#     assert set(bucket.storage) == set([0])

#     actual_unq_classes = np.unique(picks, return_counts=True)[1]

#     # Each class should be picked equal amount times
#     assert len(set(actual_unq_classes)) == 1


@pytest.mark.parametrize("cpp", [1, 5, 10])
def test_partition_distribution(cpp):
    # Prepare
    num_partitions = 10
    num_classes = 10
    num_sections = cpp * num_partitions
    num_per_class = num_sections / num_classes

    # Execute
    p_dist = cpp_partition_distribution(
        num_classes=num_classes, num_partitions=num_partitions, cpp=cpp
    )

    # Assert
    assert len(p_dist) == num_partitions

    for partition in p_dist:
        non_zero = np.count_nonzero(partition)
        assert non_zero == cpp

    assert np.sum(p_dist) == num_sections

    # Each class should occur globally same number of times
    for column in p_dist.T:
        assert np.sum(column) == num_per_class
