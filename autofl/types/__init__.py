from typing import List, Tuple

from numpy import ndarray

# Returned from keras
KerasDataset = Tuple[Tuple[ndarray, ndarray], Tuple[ndarray, ndarray]]

# User in the autofl project
NDArrayDataset = Tuple[ndarray, ndarray, ndarray, ndarray]
FederatedDataset = Tuple[List[Tuple[ndarray, ndarray]], Tuple[ndarray, ndarray]]

FnameNDArrayTuple = Tuple[str, ndarray]
