from abc import ABC
from typing import Optional

DEFAULT_R = 50  # Number of federated learning rounds
DEFAULT_E = 5  # Number of epochs (on each client, in each round)
DEFAULT_C = 0.1  # Fraction of participants participating in each round
DEFAULT_B = 64  # Batch size


class Task(ABC):
    def __init__(
        self,
        dataset_name: str,
        model_name: str,
        R: int,
        E: int,
        C: float,
        B: int,
        partition_id: Optional[int] = None,
    ):
        self.dataset_name = dataset_name
        self.model_name = model_name
        self.R = R
        self.E = E
        self.C = C
        self.B = B
        self.partition_id = partition_id


class VisionTask(Task):
    def __init__(
        self,
        dataset_name: str,
        model_name="blog_cnn",
        R=DEFAULT_R,
        E=DEFAULT_E,
        C=DEFAULT_C,
        B=DEFAULT_B,
    ):
        super().__init__(
            dataset_name=dataset_name, model_name=model_name, R=R, E=E, C=C, B=B
        )


class UnitaryVisionTask(Task):
    def __init__(
        self,
        dataset_name: str,
        model_name="blog_cnn",
        E=DEFAULT_R * DEFAULT_E,
        B=DEFAULT_B,
        partition_id: int = 0,
    ):
        super().__init__(
            dataset_name=dataset_name,
            model_name=model_name,
            R=1,
            E=E,
            C=0.0,
            B=B,
            partition_id=partition_id,
        )
