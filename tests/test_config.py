"""
Tests for the `xain_fl.config.Config` class.
"""
# pytest's fixture pattern is based triggers pylint's
# `redefined-outer-name` lint.
# pylint: disable=redefined-outer-name
import re

import pytest

from xain_fl.config import Config, InvalidConfig


@pytest.fixture
def server_sample():
    """
    Return a valid "server" section
    """
    return {
        "host": "localhost",
        "port": 50051,
        "grpc_options": {
            "grpc.max_receive_message_length": -1,
            "grpc.max_send_message_length": -1,
        },
    }


@pytest.fixture
def ai_sample():
    """
    Return a valid "ai" section
    """
    return {
        "rounds": 1,
        "epochs": 1,
        "min_participants": 1,
        "fraction_participants": 1.0,
    }


@pytest.fixture
def storage_sample():
    """
    Return a valid "storage" section
    """
    return {
        "endpoint": "http://localhost:9000",
        "bucket": "aggregated_weights",
        "secret_access_key": "my-secret",
        "access_key_id": "my-key-id",
    }


@pytest.fixture
def config_sample(server_sample, ai_sample, storage_sample):
    """
    Return a valid config
    """
    return {
        "ai": ai_sample,
        "server": server_sample,
        "storage": storage_sample,
    }


def test_load_valid_config(config_sample):
    """
    Check that a valid config is loaded correctly
    """
    config = Config.from_unchecked_dict(config_sample)

    assert config.server.host == "localhost"
    assert config.server.port == 50051

    assert config.server.grpc_options == [
        ("grpc.max_receive_message_length", -1),
        ("grpc.max_send_message_length", -1),
    ]

    assert config.ai.rounds == 1
    assert config.ai.epochs == 1
    assert config.ai.min_participants == 1
    assert config.ai.fraction_participants == 1.0

    assert config.storage.endpoint == "http://localhost:9000"
    assert config.storage.bucket == "aggregated_weights"
    assert config.storage.secret_access_key == "my-secret"
    assert config.storage.access_key_id == "my-key-id"


def test_server_config_ip_address(config_sample, server_sample):
    """Check that the config is loaded correctly when the `server.host`
    key is an IP address

    """
    # Ipv4 host
    server_sample["host"] = "1.2.3.4"
    config_sample["server"] = server_sample
    config = Config.from_unchecked_dict(config_sample)
    assert config.server.host == server_sample["host"]

    # Ipv6 host
    server_sample["host"] = "::1"
    config_sample["server"] = server_sample
    config = Config.from_unchecked_dict(config_sample)
    assert config.server.host == server_sample["host"]


def test_server_config_extra_key(config_sample, server_sample):
    """Check that the config is rejected when the server section contains
    an extra key

    """
    server_sample["extra-key"] = "foo"
    config_sample["server"] = server_sample

    with AssertInvalid() as err:
        Config.from_unchecked_dict(config_sample)

    err.check_section("server")
    err.check_extra_key("extra-key")


def test_server_config_invalid_host(config_sample, server_sample):
    """Check that the config is rejected when the `server.host` key is
    invalid.

    """
    server_sample["host"] = 1.0
    config_sample["server"] = server_sample

    with AssertInvalid() as err:
        Config.from_unchecked_dict(config_sample)

    err.check_other(
        re.compile("Invalid `server.host`: value must be a valid domain name or IP address")
    )


def test_server_config_valid_ipv6(config_sample, server_sample):
    """Check some edge cases with IPv6 `server.host` key"""
    server_sample["host"] = "::"
    config_sample["server"] = server_sample
    config = Config.from_unchecked_dict(config_sample)
    assert config.server.host == server_sample["host"]

    server_sample["host"] = "fe80::"
    config_sample["server"] = server_sample
    config = Config.from_unchecked_dict(config_sample)
    assert config.server.host == server_sample["host"]


# Adapted from unittest's assertRaises
class AssertInvalid:
    """A context manager that checks that an `xainfl.config.InvalidConfig`
    exception is raised, and provides helpers to perform checks on the
    exception.

    """

    def __init__(self):
        self.message = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, _tb):
        if exc_type is None:
            raise Exception("Did not get an exception")
        if not isinstance(exc_value, InvalidConfig):
            # let this unexpected exception be re-raised
            return False

        self.message = str(exc_value)

        return True

    def check_section(self, section):
        """Check that the error message mentions the given section of the
        configuration file.

        """

        needle = re.compile(f"Key '{section}' error:")
        assert re.search(needle, self.message)

    def check_extra_key(self, key):
        """Check that the error mentions the given configuration key"""
        needle = re.compile(f"Wrong key '{key}' in")
        assert re.search(needle, self.message)

    def check_other(self, needle):
        """Check that the error message matches the given pattern.

        """
        assert re.search(needle, self.message)
