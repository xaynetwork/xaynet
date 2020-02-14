"""XAIN FL tests for metric store"""
import json
from unittest import mock

from influxdb import InfluxDBClient
import pytest

from xain_fl.config import MetricsConfig
from xain_fl.coordinator.metrics_store import MetricsStore, MetricsStoreError


@pytest.fixture()
def empty_json_participant_metrics_sample():
    """Return a valid metric object."""
    return json.dumps([])


@pytest.fixture()
def invalid_json_participant_metrics_sample():
    """Return a invalid metric object."""
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
def test_valid_participant_metrics(
    write_points_mock, participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that write_points does not raise an exception on a valid metric object."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_participant_metrics(participant_metrics_sample)
    write_points_mock.assert_called_once()


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_points_exception_handling_write_participant_metrics(
    write_points_mock, participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the
    write_participant_metrics method."""

    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_participant_metrics(participant_metrics_sample)


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_invalid_json_exception_handling(write_points_mock):
    """Check that raised exceptions of the write_points method are caught in the
    write_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_participant_metrics('{"a": 1')
    write_points_mock.assert_not_called()

    with pytest.raises(MetricsStoreError):
        metric_store.write_participant_metrics("{1: 1}")
    write_points_mock.assert_not_called()


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_empty_metrics_exception_handling(
    write_points_mock, empty_json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the
    write_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_participant_metrics(empty_json_participant_metrics_sample)
    write_points_mock.assert_not_called()


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_invalid_schema_exception_handling(
    write_points_mock, invalid_json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the
    write_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_participant_metrics(invalid_json_participant_metrics_sample)
    write_points_mock.assert_not_called()


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_valid_coordinator_metrics(
    write_points_mock, coordinator_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that write_points does not raise an exception on a valid metric object."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_coordinator_metrics(coordinator_metrics_sample, tags={"1": "2"})
    write_points_mock.assert_called_once()


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_points_exception_handling_write_coordinator_metrics(
    write_points_mock, coordinator_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are caught in the
    write_coordinator_metrics method."""

    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_coordinator_metrics(coordinator_metrics_sample)
