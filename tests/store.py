"""A mocked S3 store to store data in memory."""

from collections import defaultdict
import pickle
import typing

import numpy as np
from xain_sdk.store import S3GlobalWeightsReader, S3LocalWeightsWriter

from xain_fl.config import StorageConfig
from xain_fl.coordinator.store import S3GlobalWeightsWriter, S3LocalWeightsReader


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
    def Bucket(self, _name: str) -> "MockS3Resource":  # pylint: disable=invalid-name
        """Mock of the `boto3.S3.Client.Bucket` method.

        Args:
            _name: Name of the bucket (un-used).

        Returns:
            The bucket itself.
        """

        return self

    # The names come from the `boto3` API we're mocking
    def put_object(self, Body: bytes, Key: str) -> None:  # pylint: disable=invalid-name
        """Mock of the `boto3.S3.Client.put_object` method.

        Args:
            Body: Data to write to the bucket.
            Key: Key under which the data should be stored.
        """

        # We store the data non-serialized, to make it easier to check it.
        self.fake_store[Key] = pickle.loads(Body)
        self.writes[Key] += 1

    def download_fileobj(self, key: str, buf: typing.IO) -> None:
        """Mock of the `boto3.S3.Client.download_fileobj` method.

        Args:
            key: Key under which the data to retrieve is stored.
            buf: Buffer for writing the data.
        """

        data = pickle.dumps(self.fake_store[key])
        buf.write(data)
        self.reads[key] += 1


class MockS3Coordinator(S3GlobalWeightsWriter, S3LocalWeightsReader):
    """A mocked S3 store for the coordinator.

    A partial mock of the ``xain-fl.coordinator.store.S3GlobalWeightsWriter`` and
    ``xain-fl.coordinator.store.S3LocalWeightsReader`` class that does not perform any
    IO. Instead, data is stored in memory.
    """

    # We DO NOT want to call the parent class __init__, since it tries
    # to initialize a connection to a non-existent external resource
    def __init__(self, mock_s3_resource):
        self.config = StorageConfig(
            endpoint="endpoint",
            access_key_id="access_key_id",
            secret_access_key="secret_access_key",
            bucket="bucket",
        )
        self.s3 = mock_s3_resource

    def assert_read(self, participant_id: str, round: int) -> None:
        """Check that the local weights for participant at round were read exactly once.

        Args:
            participant_id: The ID of the participant.
            round: The number of the round.
        """

        key = f"{round}/{participant_id}"
        reads = self.s3.reads[key]
        assert reads == 1, f"got {reads} reads for round {key}, expected 1"

    def assert_wrote(self, round: int, weights: np.ndarray) -> None:
        """Check that the given weights have been written to the store for the round.

        Args:
            round: Round to which the weights belong.
            weights: Weights to store.
        """

        writes = self.s3.writes[f"{round}/global"]
        # Under normal conditions, we should write data exactly once
        assert writes == 1, f"got {writes} writes for round {round}, expected 1"
        np.testing.assert_array_equal(self.s3.fake_store[f"{round}/global"], weights)

    def assert_didnt_write(self, round: int) -> None:
        """Check that the weights for the round have NOT been written to the store.

        Args:
            round: Round to which the weights belong.
        """

        assert self.s3.writes[f"{round}/global"] == 0


class MockS3Participant(S3LocalWeightsWriter, S3GlobalWeightsReader):
    """A mocked S3 store for the participant.
    
    A partial mock of the ``xain_sdk.store.S3GlobalWeightsReader`` and
    ``xain_sdk.store.S3LocalWeightsWriter`` class that does not perform any IO.
    Instead, data is stored in memory.
    """

    def __init__(self, mock_s3_resource):
        self.config = StorageConfig(
            endpoint="endpoint",
            access_key_id="access_key_id",
            secret_access_key="secret_access_key",
            bucket="bucket",
        )
        self.s3 = mock_s3_resource

    def assert_wrote(
        self, participant_id: str, round: int, weights: np.ndarray
    ) -> None:
        """Check that the weights have been written to the store for the round.

        Args:
            participant_id: ID of the participant.
            round: Round to which the weights belong.
            weights: Weights to store.
        """

        key = f"{round}/{participant_id}"
        writes = self.s3.writes[key]
        assert writes == 1, f"got {writes} writes for {key}, expected 1"
        np.testing.assert_array_equal(self.s3.fake_store[key], weights)

    def assert_didnt_write(self, participant_id: str, round: int) -> None:
        """Check that the weights for the round have NOT been written to the store.

        Args:
            participant_id: ID of the participant.
            round: Round to which the weights belong.
        """

        key = f"{round}/{participant_id}"
        assert self.s3.writes[key] == 0
