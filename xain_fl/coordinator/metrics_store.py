# pylint: disable=missing-docstring
from abc import ABC, abstractmethod
from datetime import datetime
from typing import Dict

from influxdb import InfluxDBClient
from numpy import ndarray

from xain_fl.config import MetricsConfig


def current_time():
    return datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")


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
        # pylint: disable=invalid-name
        self.influx_client = InfluxDBClient(
            self.config.host,
            self.config.port,
            self.config.user,
            self.config.password,
            self.config.db_name,
        )

    def write_metrics(self, participant_id: str, metrics: Dict[str, ndarray]) -> bool:
        metrics = {
            "measurement": "participant",
            "tags": {"id": participant_id},
            "time": current_time(),
            # "fields": {"state": state},
        }

        return self.influx_client.write_points([metrics])
