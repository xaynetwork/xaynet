"""XAIN FL Metric Store"""

from abc import ABC, abstractmethod
from datetime import datetime, timedelta
from typing import Dict, List

from influxdb import InfluxDBClient
from numpy import array2string, ndarray

from xain_fl.config import MetricsConfig
from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


class AbstractMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        """Write the participant metrics on behalf of the participant with the given participant_id
        into a metric store.

        Args:

            participant_id: The ID of the participant.
            metrics: The metrics of the participant with the given participant_id.

        Returns:

            True, on success, otherwise False.
        """


class DummyMetricsStore(AbstractMetricsStore):  # pylint: disable=too-few-public-methods
    """A metric store that does nothing."""

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        """A dummy method that has no effect.

        Args:

            participant_id: The ID of the participant.
            metrics: The metrics of the participant with the given participant_id.

        Returns:

            True, on success, otherwise False.
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

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        """Write the participant metrics on behalf of the participant with the given participant_id
        into InfluxDB.

        Args:

            participant_id: The ID of the participant.
            metrics: The metrics of the participant with the given participant_id.

        Returns:

            True, on success, otherwise False.
        """

        # FIXME: We will change the data format of the metrics message in a separate ticket.
        # The goal is, that coordinator doesn't need to transform the metrics anymore.
        influx_data_points = transform_metrics_to_influx_data_points(participant_id, metrics)

        try:
            return self.influx_client.write_points(influx_data_points)
        except Exception as err:  # pylint: disable=broad-except
            logger.warn("Can not write metrics", error=str(err))
            return False


def transform_metrics_to_influx_data_points(
    participant_id: str, metrics: Dict[str, ndarray]
) -> List[dict]:
    """Transform the metrics of a participant into InfluxDB data points.

    Arguments:
        participant_id: The ID of the participant.
        metrics: The metrics of the participant with the given participant_id.

    Returns:
        The metrics of the participant as InfluxDB data points.
    """

    # Currently the metrics message does not contain any timestamps.
    # We set a timestamp for each epoch data point with an interval of 1 sec.
    first_epoch_time = datetime.now()
    data_points: List = []

    for metric_name, epoch_data_points in metrics.items():
        next_epoch_time = first_epoch_time

        for epoch_data_point in epoch_data_points:
            data_point = {
                "measurement": f"participant.ai.{metric_name}",
                "tags": {"id": participant_id},
                "time": next_epoch_time,
                "fields": {
                    metric_name: array2string(
                        epoch_data_point, precision=8, suppress_small=True, floatmode="fixed"
                    )
                },
            }

            data_points.append(data_point)
            next_epoch_time += timedelta(seconds=1)

    return data_points
