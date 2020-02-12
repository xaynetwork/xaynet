"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
import json

from influxdb import InfluxDBClient
from jsonschema import validate
from structlog import get_logger

from xain_fl.config import MetricsConfig
from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


class AbstractMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_metrics(self, metrics_as_json: str):
        """Write the participant metrics on behalf of the participant into a metric store.

        Args:

            metrics_as_json: The metrics of a specific participant.

        Raises:

            MetricsStoreError: If the writing of the metrics has failed.
        """


class NullObjectMetricsStore(
    AbstractMetricsStore
):  # pylint: disable=too-few-public-methods
    """A metric store that does nothing."""

    def write_metrics(self, metrics_as_json: str):
        """A method that has no effect.

        Args:

            metrics_as_json: The metrics of a specific participant.
        """


class MetricsStore(AbstractMetricsStore):  # pylint: disable=too-few-public-methods
    """A metric store that uses InfluxDB to store the metrics."""

    def __init__(self, config: MetricsConfig):
        self.config = config
        self.influx_client = InfluxDBClient(
            self.config.host,
            self.config.port,
            self.config.user,
            self.config.password,
            self.config.db_name,
        )
        self.schema = {
            "type": "array",
            "items": [
                {
                    "type": "object",
                    "properties": {
                        "measurement": {"type": "string"},
                        "time": {"type": "number"},
                        "tags": {
                            "type": "object",
                            "additionalProperties": {"type": "string"},
                        },
                        "fields": {
                            "type": "object",
                            "additionalProperties": {"type": ["number", "string"]},
                        },
                    },
                    "required": ["measurement", "time", "fields"],
                }
            ],
            "minItems": 1,
        }

    def write_metrics(self, metrics_as_json: str):
        """Write the participant metrics on behalf of the participant into InfluxDB.

        Args:

            metrics_as_json: The metrics of a specific participant.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

        try:
            metrics = json.loads(metrics_as_json)
            validate(instance=metrics, schema=self.schema)
            self.influx_client.write_points(metrics)
        except Exception as err:  # pylint: disable=broad-except
            logger.error("Exception", err=repr(err))
            raise MetricsStoreError("Can not write metrics.") from err


class MetricsStoreError(Exception):
    """
    Raised when the writing of the metrics has failed.
    """
