import pytest


@pytest.fixture
def mock_datasets_repository() -> str:
    return "https://s3.eu-central-1.amazonaws.com/datasets.xain.io/autofl"
