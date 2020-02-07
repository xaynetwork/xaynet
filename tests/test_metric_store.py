"""XAIN FL tests for metric store"""

from unittest import mock

from influxdb import InfluxDBClient
import pytest

from xain_fl.config import MetricsConfig
from xain_fl.coordinator.metrics_store import MetricsStore, MetricsStoreError


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_metrics_exception_handling(
    write_points_mock, metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics(metrics_sample)
