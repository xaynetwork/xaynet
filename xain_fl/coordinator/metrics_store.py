# pylint: disable=missing-docstring
from abc import ABC, abstractmethod
from typing import Dict, List

from influxdb import InfluxDBClient
from numpy import ndarray

from xain_fl.config import MetricsConfig


def current_time():
    return datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


class ABCMetricsStore(ABC):
    """An abstract participant for federated learning."""

    @abstractmethod
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> None:
        """
        """


class NoMetricsStore(ABCMetricsStore):
    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> None:
        pass


class MetricsStore(ABCMetricsStore):
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

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]):
        metrics = {
            "measurement": "participant",
            "tags": {"host": participant_id},
            "time": current_time(),
            # "fields": {"state": state},
        }

        return self.influx_client.write_points([metrics])
