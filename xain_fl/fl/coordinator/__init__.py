"""Contains coordinator logic which runs the central server in federated
learning. It consists of the following modules:

    - aggregate
    - controller
"""

from xain_fl.fl.coordinator.aggregate import Aggregator
from xain_fl.fl.coordinator.controller import RandomController
