"""Contains coordinator logic which runs the central server in federated
learning. It consists of the following modules:

    - coordinator
    - aggreate
    - controller
    - evaluator
"""

from .aggregate import Aggregator
from .controller import RandomController, RoundRobinController
from .coordinator import Coordinator
