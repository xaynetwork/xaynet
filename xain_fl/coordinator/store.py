"""This module provides classes for weights storage. It currently only
works with services that provide the AWS S3 APIs.

"""
import abc
from io import BytesIO
import pickle

import boto3
from numpy import ndarray

from xain_fl.config import StorageConfig


class AbstractStore(abc.ABC):

    """An abstract class that defines the API a store must implement.

    """

    @abc.abstractmethod
    def write_weights(self, round: int, weights: ndarray) -> None:
        """Store the given `weights`, corresponding to the given `round`.

        Args:

            round: round number the weights correspond to
            weights: weights to store

        """

    @abc.abstractmethod
    def read_weights(self, participant_id: str, round: int) -> ndarray:
        """Read the weights computed by the given participant for the given
        round.

        Args:

            participant_id: ID of the participant's weights
            round: round number the weights correspond to

        """


class DummyStore(AbstractStore):
    """A store that does nothing"""

    def write_weights(self, _round: int, _weights: ndarray) -> None:
        """A dummy method that has no effect.

        Args:

            _round: round number the weights correspond to. Not used.
            _weights: weights to store. Not used.

        """

    def read_weights(self, _participant_id: str, _round: int) -> ndarray:
        """A dummy method that has no effect.

        Args:

            _participant_id: ID of the participant's weights
            _round: round number the weights correspond to

        """


class S3Store(AbstractStore):
    """A store for services that offer the AWS S3 API.

    Args:

        config: the storage configuration (endpoint URL, credentials,
            etc.)

    """

    def __init__(self, config: StorageConfig):
        self.config = config
        # pylint: disable=invalid-name
        self.s3 = boto3.resource(
            "s3",
            endpoint_url=self.config.endpoint,
            aws_access_key_id=self.config.access_key_id,
            aws_secret_access_key=self.config.secret_access_key,
            # FIXME: not sure what this should be for now
            region_name="dummy",
        )

    def write_weights(self, round: int, weights: ndarray):
        """Store the given `weights`, corresponding to the given `round`.

        Args:

            round: round number the weights correspond to
            weights: weights to store

        """
        bucket = self.s3.Bucket(self.config.bucket)
        bucket.put_object(Body=pickle.dumps(weights), Key=str(round))

    def read_weights(self, _participant_id: str, round: int) -> ndarray:
        """Download the weights computed by the given participant for the given
        round, from an S3 bucket.

        Args:

            _participant_id: ID of the participant's weights
            round: round number the weights correspond to

        Return:

            The weights read from the S3 bucket

        """
        bucket = self.s3.Bucket(self.config.bucket)
        data = BytesIO()
        bucket.download_fileobj(str(round), data)
        # FIXME: not sure whether getvalue() copies the data. If so we
        # should probably prefer getbuffer() instead.
        return pickle.loads(data.getvalue())
