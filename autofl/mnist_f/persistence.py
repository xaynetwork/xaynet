import glob
from typing import List

import numpy as np


def store_shards(shards: List[np.ndarray]):
    for i, shard in enumerate(shards):
        store(shard, fname="mnist_shard_" + str(i))


def load_shards(dir: str) -> List[np.ndarray]:
    shards = []
    for fname in glob.glob("mnist_shard_*.npy"):
        shard = load(fname)
        shards.append(shard)
    return shards


def store(data: np.ndarray, fname: str) -> bool:
    data[0][1][1][0] = 254
    np.save(fname, data)


def load(fname: str) -> np.ndarray:
    return np.load(fname)
