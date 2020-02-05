"""This module provide a subclass of
`xain_fl.coordinator.store.Store` that stores data in memory.
"""
from collections import defaultdict
import pickle
import typing

import numpy as np

from xain_fl.config import StorageConfig
from xain_fl.coordinator.store import S3GlobalWeightsWriter


class MockS3Resource:
    """Mock of the `xain-fl.coordinator.Store.s3` attribute.

    This class offers the same API than `boto3.S3.Client.bucket` but
    writes data in memory and keeps track of the reads and writes.
    """

    def __init__(self):
        # fake store where data gets written to
        self.fake_store = {}
        # count the writes for each key in the store
        self.writes = defaultdict(lambda: 0)
        # count the reads for each key in the store
        self.reads = defaultdict(lambda: 0)

    # The names come from the `boto3` API we're mocking
    # pylint: disable=invalid-name
    def Bucket(self, _name: str):
        """Mock of the `boto3.S3.Client.Bucket` method.

        Args:
            _name (str): name of the bucket (un-used)
        """
        return self

    # The names come from the `boto3` API we're mocking
    # pylint: disable=invalid-name
    def put_object(self, Body: bytes, Key: str):
        """Mock of the `boto3.S3.Client.put_object` method.

        Args:
            Body (bytes): data to write to the bucket
            Key (str): key under which the data should be stored
        """
        # We store the data non-serialized, to make it easier to
        # check it.
        self.fake_store[Key] = pickle.loads(Body)
        self.writes[Key] += 1

    def download_fileobj(self, key: str, buf: typing.IO):
        """Mock of the `boto3.S3.Client.download_fileobj` method.

        Args:
            key (str): key under which the data to retrieve is stored
            buf (bytes of file-like object): buffer for writing the data
        """
        data = pickle.dumps(self.fake_store[key])
        buf.write(data)
        self.reads[key] += 1


class MockS3Writer(S3GlobalWeightsWriter):
    """A partial mock of the
    ``xain-fl.coordinator.store.S3GlobalWeightsWriter`` class that
    does not perform any IO. Instead, data is stored in memory.

    """

    # We DO NOT want to call the parent class __init__, since it tries
    # to initialize a connection to a non-existent external resource
    #
    # pylint: disable=super-init-not-called
    def __init__(self):
        self.config = StorageConfig(
            endpoint="endpoint",
            access_key_id="access_key_id",
            secret_access_key="secret_access_key",
            global_weights_bucket="bucket",
            local_weights_bucket="bucket",
        )
        self.s3 = MockS3Resource()

    def assert_wrote(self, round: int, weights: np.ndarray):
        """Check that the given weights have been written to the store for the
given round.

        Args:
            weights (np.ndarray): weights to store
            round (int): round to which the weights belong
        """
        writes = self.s3.writes[str(round)]
        # Under normal conditions, we should write data exactly once
        assert writes == 1, f"got {writes} writes for round {round}, expected 1"
        # If the arrays contains `NaN` we cannot compare them, so we
        # replace them by zeros to do the comparison
        stored_array = np.nan_to_num(self.s3.fake_store[str(round)])
        expected_array = np.nan_to_num(weights)
        assert np.array_equal(stored_array, expected_array)

    def assert_didnt_write(self, round: int):
        """Check that the weights for the given round have NOT been written to the store.

        Args:
            round (int): round to which the weights belong

        """
        assert self.s3.writes[str(round)] == 0
