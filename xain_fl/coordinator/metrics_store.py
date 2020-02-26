"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
import json
from json import JSONDecodeError
import time
from typing import Dict, List, Optional, Union

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
        """Write the participant metrics on their behalf into a metric store.

        Args:
            metrics_as_json: The metrics of a specific participant.
        """

    @abstractmethod
    def write_metrics(
        self,
        owner: str,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """Write metrics into a metric store.

        Args:
            owner: The name of the owner of the metrics e.g. coordinator or participant.
            metrics: A dictionary with the metric names as keys and the metric values as
                values.
            tags: A dictionary to append optional metadata to the metric. Defaults to
                None.
        """


class NullObjectMetricsStore(AbstractMetricsStore):
    """A metric store that does nothing."""

    def write_received_participant_metrics(self, metrics_as_json: str) -> None:
        """A method that has no effect.

        Args:
            metrics_as_json: The metrics of a specific participant.
        """

    def write_metrics(
        self,
        owner: str,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """A method that has no effect.

        Args:
            owner: The name of the owner of the metrics e.g. coordinator or participant.
            metrics: A dictionary with the metric names as keys and the metric values as
                values.
            tags: A dictionary to append optional metadata to the metric. Defaults to
                None.
        """


class MetricsStore(AbstractMetricsStore):
    """A metric store that uses InfluxDB to store the metrics.

    Args:
        config: A metric configuration.
        influx_client: An InfluxDB database client.
        schema: A schema to validate incomming metrics.
    """

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
        """Write the participant metrics on behalf of the participant into InfluxDB.

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

    def write_metrics(
        self,
        owner: str,
        metrics: Dict[str, Union[str, int, float]],
        tags: Optional[Dict[str, str]] = None,
    ) -> None:
        """Write the metrics to InfluxDB that are collected on the coordinator site.

        Args:
            owner: The name of the owner of the metrics e.g. coordinator or participant.
            metrics: A dictionary with the metric names as keys and the metric values as
                values.
            tags: A dictionary to append optional metadata to the metric. Defaults to
                None.
        """

        if not tags:
            tags = {}

        current_time: int = int(time.time() * 1_000_000_000)
        influx_data_point = {
            "measurement": owner,
            "time": current_time,
            "tags": tags,
            "fields": metrics,
        }

        self._write_metrics([influx_data_point])

    def _write_metrics(self, influx_points: List[dict]) -> None:
        """Write the metrics to InfluxDB that are collected on the coordinator site.

        Args:
            influx_points: InfluxDB data points.

        Raises:
            MetricsStoreError: If the writing of the metrics to InfluxDB has failed.
        """

        try:
            self.influx_client.write_points(influx_points)
        except Exception as err:  # pylint: disable=broad-except
            logger.error("Exception", error=repr(err))
            raise MetricsStoreError("Can not write metrics.") from err


class MetricsStoreError(Exception):
    """Raised when the writing of the metrics has failed."""
