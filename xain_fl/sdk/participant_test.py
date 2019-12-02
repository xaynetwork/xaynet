import pytest

from . import participant
from .use_case import UseCase


@pytest.mark.xfail
def test_start():
    class MyUseCase(UseCase):
        def __init__(self, model, *args, **kwargs):
            super().__init__(model, *args, **kwargs)
            self.model = model

        def set_weights(self, weights):
            pass

        def get_weights(self):
            pass

        def train(self):
            pass

    my_use_case = MyUseCase(model={})

    participant.start(coordinator_url="http://localhost:8601", use_case=my_use_case)
