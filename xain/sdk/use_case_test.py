from .use_case import UseCase


def test_UseCase():
    class MyUseCase(UseCase):
        def __init__(self):
            super().__init__(self)

        def set_weights(self, weights):
            pass

        def get_weights(self):
            pass

        def train(self):
            pass

    MyUseCase()
