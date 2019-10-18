from typing import Dict, List, Tuple

import pytest
from numpy import ndarray

from .use_case import UseCase


def test_UseCase():
    class MyUseCase(UseCase):
        def __init__():
            super()

        def set_weights(self, weights: List[ndarray]) -> None:
            pass

        def get_weights(self) -> List[ndarray]:
            pass

        def train(self):
            pass
