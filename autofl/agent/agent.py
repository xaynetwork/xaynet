from pprint import pformat
from typing import List


def main(_):
    print("Hello, architecture search!")
    # Hardcoded architecture
    arch = Architecture()
    arch.add_layer([0])
    arch.add_layer([3, 0])
    arch.add_layer([0, 1, 0])
    arch.add_layer([2, 0, 0, 1])
    arch.add_layer([2, 0, 0, 0, 0])
    arch.add_layer([3, 1, 1, 0, 1, 0])
    print("Architecture:")
    print("\t architecture:", arch)
    print("\t num_layers:  ", arch.get_num_layers())


class Architecture:
    def __init__(self):
        self.arch: List[List[int]] = []

    def __repr__(self) -> str:
        return pformat(self.arch, indent=2)

    def get_num_layers(self) -> int:
        return len(self.arch)

    def add_layer(self, layer: List[int]) -> None:
        assert len(self.arch) == len(layer[1:])
        self.arch.append(layer)

    def get_layer(self, index: int) -> List[int]:
        assert index < len(self.arch)
        return self.arch[index]


def parse_arch_str(arch_str: str) -> Architecture:
    arch_strs: List[str] = arch_str.split()
    arch_ints: List[int] = list(map(int, arch_strs))
    arch = Architecture()
    take = 1
    while len(arch_ints) >= take:
        next_layer = arch_ints[0:take]
        arch.add_layer(next_layer)
        arch_ints = arch_ints[take:]
        take += 1
        if arch_ints:
            assert not len(arch_ints) < take
    return arch
