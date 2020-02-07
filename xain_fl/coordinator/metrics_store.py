"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
import json

from influxdb import InfluxDBClient

from xain_fl.config import MetricsConfig


class AbstractMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_metrics(self, metrics_as_json: str):
        """Write the participant metrics on behalf of the participant with the given participant_id
        into a metric store.

        Args:

            metrics_as_json: The metrics of the participant with the given participant_id.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB failed.
        """


class NullObjectMetricsStore(
    AbstractMetricsStore
):  # pylint: disable=too-few-public-methods
    """A metric store that does nothing."""

    def write_metrics(self, metrics_as_json: str):
        """A method that has no effect.

        Args:

            metrics_as_json: The metrics of the participant with the given participant_id.
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

    def write_metrics(self, metrics_as_json: str):
        """Write the participant metrics on behalf of the participant with the given participant_id
        into InfluxDB.

        Args:

            metrics_as_json: The metrics of the participant with the given participant_id.

        Raises:

            MetricsStoreError: If the writing of the metrics to InfluxDB failed.
        """

        metrics = json.loads(metrics_as_json)

        try:
            self.influx_client.write_points(metrics)
        except Exception as err:  # pylint: disable=broad-except
            raise MetricsStoreError("Can not write metrics.") from err


class MetricsStoreError(Exception):
    """
    Raised when the writing of the metrics failed.
    """
