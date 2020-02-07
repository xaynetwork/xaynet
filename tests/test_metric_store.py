"""XAIN FL tests for metric store"""
# pylint: disable=redefined-outer-name
import json
from unittest import mock

from influxdb import InfluxDBClient
import pytest

from xain_fl.config import MetricsConfig
from xain_fl.coordinator.metrics_store import MetricsStore, MetricsStoreError


@pytest.fixture()
def metrics_sample():
    """Return a valid metric object."""
    return json.dumps(
        {
            "measurement": "CPU utilization",
            "time": "00:00:00",
            "tags": {"id": "127.0.0.1:1345"},
            "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
        }
    )


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_metrics_exception_handling(metrics_sample):
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics(metrics_sample)
