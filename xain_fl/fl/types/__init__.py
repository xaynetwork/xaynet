"""XAIN FL types"""

from typing import Dict, List, Tuple

from numpy import ndarray

Theta = List[ndarray]
# TODO: (XP-241) Remove once participant was removed
History = Dict[str, List[float]]

# TODO: (XP-241) Remove once participant was removed
VolumeByClass = List[int]
Metrics = Tuple[int, VolumeByClass]
