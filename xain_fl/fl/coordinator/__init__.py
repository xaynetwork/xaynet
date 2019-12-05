"""Contains coordinator logic which runs the central server in federated
learning. It consists of the following modules:

    - coordinator
    - aggregate
    - controller
    - evaluator
"""

from .aggregate import Aggregator
from .controller import RandomController
from .coordinator import Coordinator
