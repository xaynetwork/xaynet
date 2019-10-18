from typing import Dict, List, Tuple

import pytest
from numpy import ndarray

from . import participant


def test_start_fail():
    with pytest.raises(Exception):
        participant.start()


def test_start():
    class MyUseCase:
        def set_weights():
            pass

        def get_weights():
            pass

        def train():
            pass

    my_use_case = MyUseCase()

    participant.start(coordinator_url="http://localhost:8601", use_case=my_use_case)
