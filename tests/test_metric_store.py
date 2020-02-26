"""XAIN FL tests for metric store"""
import json
from unittest import mock

from influxdb import InfluxDBClient
import pytest
from xain_proto.fl.coordinator_pb2 import State

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


@pytest.fixture()
def participant_metrics_sample():
    """Return a valid metric object."""
    return {"state": State.FINISHED}


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_received_participant_metrics(
    write_points_mock, json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Test test_write_received_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_received_participant_metrics(json_participant_metrics_sample)
    write_points_mock.assert_called_with(
        [
            {
                "measurement": "participant",
                "time": 1582017483 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
            {
                "measurement": "participant",
                "time": 1582017484 * 1_000_000_000,
                "tags": {"id": "127.0.0.1:1345"},
                "fields": {"CPU_1": 90.8, "CPU_2": 90, "CPU_3": "23", "CPU_4": 0.00,},
            },
        ]
    )


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_received_participant_metrics_write_points_exception(
    write_points_mock, json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_received_participant_metrics method."""

    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_received_participant_metrics(json_participant_metrics_sample)


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_received_participant_metrics_invalid_json_exception(write_points_mock):
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_received_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_received_participant_metrics('{"a": 1')
    write_points_mock.assert_not_called()

    with pytest.raises(MetricsStoreError):
        metric_store.write_received_participant_metrics("{1: 1}")
    write_points_mock.assert_not_called()


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_received_participant_metrics_empty_metrics_exception(
    write_points_mock, empty_json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_received_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_received_participant_metrics(
            empty_json_participant_metrics_sample
        )
    write_points_mock.assert_not_called()


@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_received_participant_metrics_invalid_schema_exception(
    write_points_mock, invalid_json_participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_received_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    with pytest.raises(MetricsStoreError):
        metric_store.write_received_participant_metrics(
            invalid_json_participant_metrics_sample
        )
    write_points_mock.assert_not_called()


@mock.patch("xain_fl.coordinator.metrics_store.time.time", return_value=1582017483.0)
@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_coordinator_metrics(
    write_points_mock, time_mock, coordinator_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Test write_coordinator_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_metrics(
        "coordinator", coordinator_metrics_sample, tags={"meta_data": "1"}
    )

    write_points_mock.assert_called_with(
        [
            {
                "measurement": "coordinator",
                "time": 1582017483 * 1_000_000_000,
                "tags": {"meta_data": "1"},
                "fields": {
                    "state": State.ROUND,
                    "round": 2,
                    "number_of_selected_participants": 0,
                },
            }
        ]
    )


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_coordinator_metrics_write_points_exception(
    write_points_mock, coordinator_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_coordinator_metrics method."""

    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics("coordinator", coordinator_metrics_sample)


@mock.patch("xain_fl.coordinator.metrics_store.time.time", return_value=1582017483.0)
@mock.patch.object(InfluxDBClient, "write_points", return_value=True)
def test_write_participant_metrics(
    write_points_mock, time_mock, participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Test write_participant_metrics method."""
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    metric_store.write_metrics(
        "participant", participant_metrics_sample, tags={"id": "1234-1234-1234"}
    )

    write_points_mock.assert_called_with(
        [
            {
                "measurement": "participant",
                "time": 1582017483 * 1_000_000_000,
                "tags": {"id": "1234-1234-1234"},
                "fields": {"state": State.FINISHED,},
            }
        ]
    )


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_participant_metrics_write_points_exception(
    write_points_mock, participant_metrics_sample,
):  # pylint: disable=redefined-outer-name,unused-argument
    """Check that raised exceptions of the write_points method are re-raised as MetricsStoreError in
    the write_participant_metrics method."""

    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )
    with pytest.raises(MetricsStoreError):
        metric_store.write_metrics("participant", participant_metrics_sample)
