"""This module provides classes for weights storage. It currently only
works with services that provide the AWS S3 APIs.

"""
import abc
from io import BytesIO
import pickle

import boto3
from numpy import ndarray

from xain_fl.config import StorageConfig


class AbstractGlobalWeightsWriter(abc.ABC):
    # pylint: disable=too-few-public-methods

    """An abstract class that defines the API for storing the aggregated
    weights the coordinator computes.

    """

    @abc.abstractmethod
    def write_weights(self, round: int, weights: ndarray) -> None:
        """Store the given `weights`, corresponding to the given `round`.

        Args:
            round: A round number the weights correspond to.
            weights: The weights to store.
        """


class AbstractLocalWeightsReader(abc.ABC):
    # pylint: disable=too-few-public-methods

    """An abstract class that defines the API for retrieving the weights
    participants upload after their training round.

    """

    @abc.abstractmethod
    def read_weights(self, participant_id: str, round: int) -> ndarray:
        """Retrieve the weights computed by the given participant for the
        given round.

        Args:
            participant_id: ID of the participant's weights.
            round: A round number the weights correspond to.
        """


class NullObjectGlobalWeightsWriter(AbstractGlobalWeightsWriter):
    # pylint: disable=too-few-public-methods
    """An implementation of ``AbstractGlobalWeightsWriter`` that does
    nothing.

    """

    def write_weights(self, round: int, weights: ndarray) -> None:
        """A dummy method that has no effect.

        Args:
            round: A round number the weights correspond to. Not used.
            weights: The weights to store. Not used.
        """


class NullObjectLocalWeightsReader(AbstractLocalWeightsReader):
    # pylint: disable=too-few-public-methods
    """An implementation of ``AbstractLocalWeightsReader`` that does
    nothing.
    """

    def read_weights(self, participant_id: str, round: int) -> ndarray:
        """A dummy method that has no effect.

        Args:
            participant_id: ID of the participant's weights. Not used.
            round: A round number the weights correspond to. Not used.
        """


class S3BaseClass:
    # pylint: disable=too-few-public-methods
    """A base class for implementating AWS S3 clients.

    Args:
        config: the storage configuration (endpoint URL, credentials, etc.)

    """

    def __init__(self, config: StorageConfig):
        self.config = config
        self.s3 = boto3.resource(  # pylint: disable=invalid-name
            "s3",
            endpoint_url=self.config.endpoint,
            aws_access_key_id=self.config.access_key_id,
            aws_secret_access_key=self.config.secret_access_key,
            # FIXME: not sure what this should be for now
            region_name="dummy",
        )


class S3GlobalWeightsWriter(AbstractGlobalWeightsWriter, S3BaseClass):
    # pylint: disable=too-few-public-methods

    """``AbstractGlobalWeightsWriter`` implementor for AWS S3 storage
    backend.

    """

    def write_weights(self, round: int, weights: ndarray):
        """Store the given `weights`, corresponding to the given `round`.

        Args:
            round: A round number the weights correspond to.
            weights: The weights to store.
        """
        bucket = self.s3.Bucket(self.config.global_weights_bucket)
        bucket.put_object(Body=pickle.dumps(weights), Key=str(round))


class S3LocalWeightsReader(AbstractLocalWeightsReader, S3BaseClass):
    # pylint: disable=too-few-public-methods

    """``AbstractLocalWeightsReader`` implementor for AWS S3 storage
    backend.

    """

    def read_weights(self, participant_id: str, round: int) -> ndarray:
        """Download the weights computed by the given participant for the given
        round, from an S3 bucket.

        Args:

            participant_id: ID of the participant's weights
            round: round number the weights correspond to

        Return:
            The weights read from the S3 bucket.
        """
        bucket = self.s3.Bucket(self.config.participants_bucket)
        data = BytesIO()
        bucket.download_fileobj(f"{participant_id}/{round}", data)
        # FIXME: not sure whether getvalue() copies the data. If so we
        # should probably prefer getbuffer() instead.
        return pickle.loads(data.getvalue())
