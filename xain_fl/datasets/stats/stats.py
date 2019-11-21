from typing import Dict, List

import numpy as np

from xain_fl.types import FederatedDataset, Partition

PartitionStat = Dict[str, List[int]]


class DSStats:
    def __init__(self, name: str, ds: FederatedDataset):
        self.name = name
        self.ds = ds

    def __repr__(self) -> str:
        width = 120
        line = "=" * width + "\n"
        output = "\nname: {}\n".format(self.name)

        all_stats = self.all()

        topic = "number_of_examples_per_label_per_shard"
        stat = all_stats[topic]

        output += "{}\n".format(topic)

        for part_index, part in stat.items():
            output += "partition: {}\t".format(part_index)
            output += "total: {}\t".format(part["total"])
            output += "per_label: {}".format(
                "\t".join([str(v).rjust(4) for v in part["per_label"]])
            )
            output += "\n"

        output += line

        return output

    def all(self) -> Dict[str, Dict[str, PartitionStat]]:
        stats = {}

        stats[
            "number_of_examples_per_label_per_shard"
        ] = self.number_of_examples_per_label_per_shard()

        return stats

    def number_of_examples_per_label_per_shard(self) -> Dict[str, PartitionStat]:
        xy_partitions, xy_val, xy_test = self.ds

        stats = {}

        zfill_width = int(np.log(len(xy_partitions)))

        ys = [y for (_, y) in xy_partitions]
        all_labels = np.unique(np.concatenate(ys, axis=0))

        for index, xy_par in enumerate(xy_partitions):
            key = str(index).zfill(zfill_width)

            stats[key] = self.number_of_examples_per_label(
                xy=xy_par, possible_labels=all_labels
            )

        stats["val"] = self.number_of_examples_per_label(
            xy=xy_val, possible_labels=all_labels
        )
        stats["test"] = self.number_of_examples_per_label(
            xy=xy_test, possible_labels=all_labels
        )

        return stats

    @staticmethod
    def number_of_examples_per_label(
        xy: Partition, possible_labels: List
    ) -> PartitionStat:
        x, y = xy

        possible_labels = list(possible_labels)
        per_label_counts = [0] * len(possible_labels)

        assert x.shape[0] == y.shape[0], "Number of examples and labels don't match"

        [unique_labels, unique_counts] = np.unique(y, return_counts=True)

        for i, l in enumerate(unique_labels):
            per_label_counts_index = possible_labels.index(l)
            per_label_counts[per_label_counts_index] = unique_counts[i]

        return {"total": x.shape[0], "per_label": per_label_counts}
