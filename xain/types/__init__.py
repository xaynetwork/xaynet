from typing import Any, Dict, List, Tuple

from numpy import ndarray

# Returned from keras
KerasDataset = Tuple[Tuple[ndarray, ndarray], Tuple[ndarray, ndarray]]

FederatedDatasetPartition = Tuple[ndarray, ndarray]
FederatedDataset = Tuple[
    List[FederatedDatasetPartition],
    FederatedDatasetPartition,
    FederatedDatasetPartition,
]

FnameNDArrayTuple = Tuple[str, ndarray]

Transition = Tuple[ndarray, Any, float, ndarray, bool]

KerasWeights = List[ndarray]
KerasHistory = Dict[str, List[float]]
