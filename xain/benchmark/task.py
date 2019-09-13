from abc import ABC
from typing import Optional

DEFAULT_R = 50  # Number of federated learning rounds
DEFAULT_E = 5  # Number of epochs (on each client, in each round)
DEFAULT_C = 0.1  # Fraction of participants participating in each round
DEFAULT_B = 64  # Batch size

DEFAULT_INSTANCE_CORES = 2
DEFAULT_TIMEOUT = 60

# pylint: disable-msg=too-many-instance-attributes
class Task(ABC):
    def __init__(
        self,
        name: str,
        dataset_name: str,
        model_name: str,
        R: int,
        E: int,
        C: float,
        B: int,
        partition_id: Optional[int] = None,
        instance_cores: int = DEFAULT_INSTANCE_CORES,
        timeout: int = DEFAULT_TIMEOUT,
        label: Optional[str] = None,
    ):
        self.name = name
        self.dataset_name = dataset_name
        self.model_name = model_name
        self.R = R
        self.E = E
        self.C = C
        self.B = B
        self.partition_id = partition_id
        self.instance_cores = instance_cores
        self.timeout = timeout
        self.label = label if label is not None else name


class VisionTask(Task):
    def __init__(
        self,
        name: str,
        dataset_name: str,
        model_name="blog_cnn",
        R=DEFAULT_R,
        E=DEFAULT_E,
        C=DEFAULT_C,
        B=DEFAULT_B,
        instance_cores=DEFAULT_INSTANCE_CORES,
        timeout: int = DEFAULT_TIMEOUT,
        label: Optional[str] = None,
    ):
        super().__init__(
            name=name,
            dataset_name=dataset_name,
            model_name=model_name,
            R=R,
            E=E,
            C=C,
            B=B,
            instance_cores=instance_cores,
            timeout=timeout,
            label=label,
        )


class UnitaryVisionTask(Task):
    def __init__(
        self,
        name: str,
        dataset_name: str,
        model_name="blog_cnn",
        E=DEFAULT_R * DEFAULT_E,
        B=DEFAULT_B,
        partition_id: int = 0,
        instance_cores=DEFAULT_INSTANCE_CORES,
        timeout: int = DEFAULT_TIMEOUT,
        label: Optional[str] = None,
    ):
        super().__init__(
            name=name,
            dataset_name=dataset_name,
            model_name=model_name,
            R=1,
            E=E,
            C=0.0,
            B=B,
            partition_id=partition_id,
            instance_cores=instance_cores,
            timeout=timeout,
            label=label,
        )
