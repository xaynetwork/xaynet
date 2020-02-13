"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
import json
import time
from typing import Optional, Union

from influxdb import InfluxDBClient
from jsonschema import validate
from structlog import get_logger

from xain_fl.config import MetricsConfig
from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


class AbstractMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_participant_metrics(self, metrics_as_json: str):
        """
        Write the participant metrics on behalf of the participant into a metric store.

        Args:

            metrics_as_json: The metrics of a specific participant.

        Raises:

            MetricsStoreError: If the writing of the metrics has failed.
        """

    @abstractmethod
    def write_coordinator_metrics(
        self, metric: str, value: Union[str, int, float], tags: Optional[dict] = None,
    ):
        """
        Write the metrics to a metric store that are collected on the coordinator site.

        Args:

            metric: The name of metric.
            value: The value of the metric.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """


class NullObjectMetricsStore(
    AbstractMetricsStore
):  # pylint: disable=too-few-public-methods
    """A metric store that does nothing."""

    def write_participant_metrics(self, metrics_as_json: str):
        """
        A method that has no effect.

        Args:

            metrics_as_json: The metrics of a specific participant.
        """

    def write_coordinator_metrics(
        self, metric: str, value: Union[str, int, float], tags: Optional[dict] = None,
    ):
        """
        A method that has no effect.

        Args:

            metric: The name of metric.
            value: The value of the metric.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.
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

    def write_participant_metrics(self, metrics_as_json: str):
        """
        Write the participant metrics on behalf of the participant into InfluxDB.

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
            raise MetricsStoreError("Can not write participant metrics.") from err

    def write_coordinator_metrics(
        self, metric: str, value: Union[str, int, float], tags: Optional[dict] = None,
    ):
        """
        Write the metrics to influxDB that are collected on the coordinator site.

        Args:

            metric: The name of metric.
            value: The value of the metric.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """
        if not tags:
            tags = {}

        current_time: int = int(time.time() * 1_000_000_000)
        metrics = {
            "measurement": metric,
            "time": current_time,
            "tags": tags,
            "fields": {metric: value},
        }

        try:
            self.influx_client.write_points([metrics])
        except Exception as err:  # pylint: disable=broad-except
            logger.error("Exception", err=repr(err))
            raise MetricsStoreError("Can not write coordinator metrics.") from err


class MetricsStoreError(Exception):
    """
    Raised when the writing of the metrics has failed.
    """
