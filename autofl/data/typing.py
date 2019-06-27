from typing import List, Tuple

from numpy import ndarray

# Returned from keras
KerasDataset = Tuple[Tuple[ndarray, ndarray], Tuple[ndarray, ndarray]]

# User in the autofl project
Dataset = Tuple[ndarray, ndarray, ndarray, ndarray]
FederatedDataset = Tuple[List[Tuple[ndarray, ndarray]], Tuple[ndarray, ndarray]]

FilenameNDArrayTuple = Tuple[str, ndarray]
