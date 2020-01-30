# pylint: disable=missing-docstring
from abc import ABC, abstractmethod
from calendar import timegm
from datetime import datetime, timedelta
from typing import Dict, List

from influxdb import InfluxDBClient
from numpy import array2string, ndarray

from xain_fl.config import MetricsConfig
from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


def current_time_in_sec():
    return timegm(datetime.utcnow().utctimetuple())


class AbstractMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        """
        Args:

            participant_ids: The list of IDs of the participants selected
                to participate in this round.
            metrics: The metrics of the participant with the given participant_id.

        Returns:

            True, on success, otherwise False.
        """


class DummyMetricsStore(AbstractMetricsStore):  # pylint: disable=too-few-public-methods
    """A metric store that does nothing."""

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        pass


class MetricsStore(AbstractMetricsStore):  # pylint: disable=too-few-public-methods
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
        # FIXME: We will change the data format of the metrics message in a separate ticket.
        # The goal is, that coordinator doesn't need to transform the metrics anymore.

        influx_data_points = transform_metrics_to_influx_data_points(participant_id, metrics)

        try:
            return self.influx_client.write_points(influx_data_points)
        except Exception as err:  # pylint: disable=broad-except
            logger.warn("Can not write metrics", error=str(err))
            return False


def format_date(total_seconds):
    return datetime.fromtimestamp(total_seconds).strftime("%Y-%m-%dT%H:%M:%SZ")


def transform_metrics_to_influx_data_points(participant_id: str, metrics: Dict[str, ndarray]):
    start_first_epoch_in_sec = current_time_in_sec()
    data_points: List = []

    for name, epoch_data_points in metrics.items():
        next_epoch_time_in_sec = timedelta(seconds=start_first_epoch_in_sec)

        for epoch_data_point in epoch_data_points:
            data_point = {
                "measurement": f"participant.ai.{name}",
                "tags": {"id": participant_id},
                "time": format_date(next_epoch_time_in_sec.total_seconds()),
                "fields": {
                    name: array2string(
                        epoch_data_point, precision=8, suppress_small=True, floatmode="fixed"
                    )
                },
            }

            data_points.append(data_point)
            next_epoch_time_in_sec += timedelta(seconds=1)

    return data_points
