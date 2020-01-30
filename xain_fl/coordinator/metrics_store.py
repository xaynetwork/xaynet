# pylint: disable=missing-docstring
from abc import ABC, abstractmethod
from datetime import datetime
from typing import Dict

from influxdb import InfluxDBClient
from numpy import ndarray

from xain_fl.config import MetricsConfig


def current_time():
    return datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


class ABCMetricsStore(ABC):  # pylint: disable=too-few-public-methods
    """An abstract metric store."""

    @abstractmethod
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> None:
        """
        Args:

        participant_ids: The list of IDs of the participants selected
            to participate in this round.
        metrics :
        """


class NoMetricsStore(ABCMetricsStore):  # pylint: disable=too-few-public-methods
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> None:
        pass


class MetricsStore(ABCMetricsStore):  # pylint: disable=too-few-public-methods
    def __init__(self, config: MetricsConfig):
        self.config = config
        # pylint: disable=invalid-name
        self.influx_client = InfluxDBClient(
            self.config.host,
            self.config.port,
            self.config.user,
            self.config.password,
            self.config.db_name,
        )

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> None:
        metrics = {
            "measurement": "participant",
            "tags": {"host": participant_id},
            "time": current_time(),
            # "fields": {"state": state},
        }

        return self.influx_client.write_points([metrics])
