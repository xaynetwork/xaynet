import numpy as np


def store(data: np.ndarray, fname: str):
    np.save(fname, data)


def load(fname: str) -> np.ndarray:
    return np.load(fname)
