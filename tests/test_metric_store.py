"""XAIN FL tests for metric store"""
import json
from unittest import mock

from influxdb import InfluxDBClient
import pytest

from xain_fl.config import MetricsConfig
from xain_fl.coordinator.metrics_store import MetricsStore, MetricsStoreError


@pytest.fixture()
def metrics_sample_empty():
    """Return a valid metric object."""
    return json.dumps([])


@pytest.fixture()
def metrics_sample_invalid():
    """Return a valid metric object."""
    return json.dumps(
        [
            {
                "measurement": "CPU utilization",
                "time": "00:00:00",
                "tags": {"id": "127.0.0.1:1345"},
            }
        ]
    )


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_valid_metric(
    write_points_mock, metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_metrics(metrics_sample)


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_points_exception_handling(
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


def test_invalid_json_exception_handling():
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics('{"a": 1')

    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics("{1: 1}")


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_empty_metrics_exception_handling(
    write_points_mock, metrics_sample_empty,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics(metrics_sample_empty)


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_invalid_schema_exception_handling(
    write_points_mock, metrics_sample_invalid,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics(metrics_sample_invalid)
