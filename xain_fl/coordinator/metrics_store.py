"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
import json
from json import JSONDecodeError
import time
from typing import Dict, Optional, Union

from influxdb import InfluxDBClient
from jsonschema import ValidationError, validate
from structlog import get_logger

from xain_fl.config import MetricsConfig
from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


class AbstractMetricsStore(ABC):
    """An abstract metric store."""

    @abstractmethod
    def write_received_participant_metrics(self, metrics_as_json: str) -> None:
        """
        Write the participant metrics on behalf of the participant into a metric store.

        Args:

            metrics_as_json: The metrics of a specific participant.

        Raises:

            MetricsStoreError: If the writing of the metrics has failed.
        """

    @abstractmethod
    def write_coordinator_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to a metric store that are collected on the coordinator site and owned by
        the coordinator.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

    @abstractmethod
    def write_participant_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to a metric store that are collected on the coordinator site and owned by
        a participant.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """


class NullObjectMetricsStore(AbstractMetricsStore):
    """A metric store that does nothing."""

    def write_received_participant_metrics(self, metrics_as_json: str) -> None:
        """
        A method that has no effect.

        Args:

            metrics_as_json: The metrics of a specific participant.
        """

    def write_coordinator_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        A method that has no effect.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.
        """

    def write_participant_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        A method that has no effect.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
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
                        "measurement": {"const": "participant"},
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

    def write_received_participant_metrics(self, metrics_as_json: str) -> None:
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
        except (ValidationError, JSONDecodeError) as err:
            logger.error("Exception", error=repr(err))
            raise MetricsStoreError("Can not write participant metrics.") from err
        else:
            self._write_metrics(metrics)

    def write_participant_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to InfluxDB that are collected on the coordinator site and owned by a
        participant.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """
        self.write_metrics("participant", metrics, tags)

    def write_coordinator_metrics(
        self,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to InfluxDB that are collected on the coordinator site and owned by the
        coordinator.

        Args:

            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

        self.write_metrics("coordinator", metrics, tags)

    def write_metrics(
        self,
        measurement: str,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Write the metrics to InfluxDB that are collected on the coordinator site.

        Args:

            measurement: The name of the measurement.
            metrics: A dictionary with the metric names as keys and the metric values as values.
            tags: A dictionary to append optional metadata to the metric. Defaults to None.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

        if not tags:
            tags = {}

        current_time: int = int(time.time() * 1_000_000_000)
        influx_data_point = {
            "measurement": measurement,
            "time": current_time,
            "tags": tags,
            "fields": metrics,
        }

        self._write_metrics(influx_data_point)

    def _write_metrics(self, influx_point: dict) -> None:
        """
        Write the metrics to InfluxDB that are collected on the coordinator site.

        Args:

            influx_point: A InfluxDB data point.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

        try:
            self.influx_client.write_points([influx_point])
        except Exception as err:  # pylint: disable=broad-except
            logger.error("Exception", error=repr(err))
            raise MetricsStoreError("Can not write metrics.") from err


class MetricsStoreError(Exception):
    """
    Raised when the writing of the metrics has failed.
    """
