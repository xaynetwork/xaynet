import numpy as np

# Passed to RandomState for predictable shuffling
SEED = 851746


class Bucket:
    def __init__(self, num_classes: int, num_per_class: int, dtype=np.int8):
        self.dtype = dtype
        self.num_class = num_classes

        # Let the bucket have its own RandomState although beware that each  bucket will have it
        # initialized the same. Given a sequence of method calls the result will always be the same
        # pylint: disable=no-member
        self.rst = np.random.RandomState(seed=SEED)

        # index == class and value == how many are left
        self.storage = np.full((num_classes), num_per_class, dtype=self.dtype)

    def zero_indicies(self):
        return np.where(self.storage == 0)[0]

    def multi_indicies(self):
        """Returns indices which have more than one section left"""
        return np.where(self.storage > 1)[0]

    def has_distinct_sections(self, num_distinct_sections: int) -> bool:
        possible_choices = np.flatnonzero(self.storage)
        return possible_choices.size >= num_distinct_sections

    def sample(self, num_distinct_sections: int):
        possible_choices = np.flatnonzero(self.storage)
        choices = self.rst.choice(
            possible_choices, num_distinct_sections, replace=False
        )
        # pylint: disable=no-member
        np.subtract.at(self.storage, choices, 1)

        return choices

    def inc_dec(self, index_inc: int, index_dec: int) -> None:
        # pylint: disable=no-member
        np.subtract.at(self.storage, [index_dec], 1)
        np.add.at(self.storage, [index_inc], 1)


def distribution(num_classes: int, num_partitions: int, cpp: int) -> np.ndarray:
    """
    :param num_classes: number of distinct unique classes
    :param num_partitions: number of partitions
    :param cpp: number of classes per partition required

    :returns: parition distribution as an ndarray of shape (num_partitions, num_classes)
              with ones at the locations where a section should be
    """
    # pylint: disable=no-member
    rst = np.random.RandomState(seed=SEED)

    dtype = np.int8
    num_sections = cpp * num_partitions

    assert num_sections % num_classes == 0
    num_per_class = num_sections // num_classes

    partitions = np.zeros((num_partitions, num_classes), dtype=dtype)

    # e.g. [20, 20, 20, 20] for num_classes=4 and cpp = 2
    bucket = Bucket(num_classes, num_per_class)

    for p in partitions:
        # 1. Check if there are 5 distinct non-zero values in the bucket
        while not bucket.has_distinct_sections(cpp):
            # Swap a section (which is available multiple times) from the bucket
            # with a section of a partition where that partition does not contain
            # the bucket section yet

            # pull from partition and fill bucket
            bucket_zero_indicies = bucket.zero_indicies()

            # pull from bucket and fill partition
            bucket_multi_indicies = bucket.multi_indicies()

            # sample one index from each
            bucket_zero_index = rst.choice(bucket_zero_indicies, 1)[0]
            bucket_multi_index = rst.choice(bucket_multi_indicies, 1)[0]

            # Find partition where:
            # - index_zero is at least one
            # - index_multi is zero
            partition_candidates_zero = np.where(partitions.T[bucket_zero_index] == 1)
            partition_candidates_multi = np.where(partitions.T[bucket_multi_index] == 0)
            partition_candidates = np.intersect1d(
                partition_candidates_zero, partition_candidates_multi
            )

            # Pick one partiton to modify
            pc_index = rst.choice(partition_candidates, 1)[0]

            # Swap
            partitions[pc_index][bucket_multi_index] += 1
            partitions[pc_index][bucket_zero_index] -= 1
            bucket.inc_dec(index_inc=bucket_zero_index, index_dec=bucket_multi_index)

        # 2. Sample from the bucket
        p_indicies = bucket.sample(num_distinct_sections=cpp)
        np.add.at(p, p_indicies, 1)

    return partitions
