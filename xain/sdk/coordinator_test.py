from typing import Dict, List, Tuple

import pytest
from numpy import ndarray

from . import coordinator


def test_start():
    coordinator.start()
