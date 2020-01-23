# pylint: disable=missing-docstring
from io import BytesIO
import pickle

import boto3
from numpy import ndarray


# pylint: disable=too-few-public-methods
class StoreConfig:
    def __init__(
        self, endpoint_url: str, access_key_id: str, secret_access_key: str, bucket: str,
    ):
        self.endpoint_url = endpoint_url
        self.access_key_id = access_key_id
        self.secret_access_key = secret_access_key
        self.bucket = bucket


class Store:
    def __init__(self, config: StoreConfig):
        self.config = config
        # pylint: disable=invalid-name
        self.s3 = boto3.resource(
            "s3",
            endpoint_url=self.config.endpoint_url,
            aws_access_key_id=self.config.access_key_id,
            aws_secret_access_key=self.config.secret_access_key,
            # FIXME: not sure what this should be for now
            region_name="dummy",
        )

    def write_weights(self, round: int, weights: ndarray):
        bucket = self.s3.Bucket(self.config.bucket)
        bucket.put_object(Body=pickle.dumps(weights), Key=str(round))

    def read_weights(self, round: int) -> ndarray:
        bucket = self.s3.Bucket(self.config.bucket)
        data = BytesIO()
        bucket.download_fileobj(str(round), data)
        # FIXME: not sure whether getvalue() copies the data. If so we
        # should probably prefer getbuffer() instead.
        return pickle.loads(data.getvalue())
