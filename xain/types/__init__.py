from typing import Any, Dict, List, Optional, Tuple

from numpy import ndarray

# Returned from keras
KerasDataset = Tuple[Tuple[ndarray, ndarray], Tuple[ndarray, ndarray]]

Partition = Tuple[ndarray, ndarray]
FederatedDataset = Tuple[List[Partition], Partition, Partition]

FnameNDArrayTuple = Tuple[str, ndarray]

Transition = Tuple[ndarray, Any, float, ndarray, bool]

Theta = List[ndarray]
History = Dict[str, List[float]]

VolumeByClass = List[int]
Metrics = Tuple[int, VolumeByClass]

PlotValues = Tuple[str, List[float], Optional[List[int]]]
XticksLocations = List[int]
XticksLabels = List[str]
