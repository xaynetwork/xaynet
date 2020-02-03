"""XAIN FL tests for metric store"""
# pylint: disable=redefined-outer-name
from unittest import mock

from influxdb import InfluxDBClient
import numpy as np
import pytest

from xain_fl.config import MetricsConfig
from xain_fl.coordinator.metrics_store import (
    MetricsStore,
    NullObjectMetricsStore,
    transform_metrics_to_influx_data_points,
)


@pytest.fixture()
def metrics_sample():
    """Return a valid metric object."""
    return {
        "metric_1": np.array([0.2, 0.44]),
        "metric_2": np.array([0.99, 0.55]),
    }


def test_null_object_metrics_store_always_return_true(metrics_sample):
    """Check that the null object metric store always retruns true."""

    no_metric_store = NullObjectMetricsStore()

    assert no_metric_store.write_metrics("participant_id", metrics_sample)


def test_transform_data(metrics_sample):
    """Check that a metric object is correctly transformed into the influx data point structure."""
    actual_data_points = transform_metrics_to_influx_data_points("participant_id", metrics_sample)
    expected_data_points = [
        {
            "measurement": "participant.ai.metric_1",
            "tags": {"id": "participant_id"},
            "fields": {"metric_1": "0.20000000"},
        },
        {
            "measurement": "participant.ai.metric_1",
            "tags": {"id": "participant_id"},
            "fields": {"metric_1": "0.44000000"},
        },
        {
            "measurement": "participant.ai.metric_2",
            "tags": {"id": "participant_id"},
            "fields": {"metric_2": "0.99000000"},
        },
        {
            "measurement": "participant.ai.metric_2",
            "tags": {"id": "participant_id"},
            "fields": {"metric_2": "0.55000000"},
        },
    ]

    for (actual_dp, expected_dp) in zip(actual_data_points, expected_data_points):
        assert actual_dp["measurement"] == expected_dp["measurement"]
        assert actual_dp["tags"] == expected_dp["tags"]
        assert actual_dp["fields"] == expected_dp["fields"]

    assert actual_data_points[0]["time"] == actual_data_points[2]["time"]
    assert actual_data_points[1]["time"] == actual_data_points[3]["time"]


@mock.patch.object(InfluxDBClient, "write_points", side_effect=Exception())
def test_write_metrics_exception_handling(metrics_sample):
    """Check that raised exceptions of the write_points method are caught in the write_metrics
    method.
    """
    metric_store = MetricsStore(
        MetricsConfig(enable=True, host="", port=1, user="", password="", db_name="")
    )

    assert not metric_store.write_metrics("participant_id", metrics_sample)
