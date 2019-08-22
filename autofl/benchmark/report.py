import json
import os
from typing import Dict, List, Optional, Tuple

import matplotlib.pyplot as plt
from absl import app, flags

FORMAT: str = "png"

FLAGS = flags.FLAGS


def get_abspath(fname: str, fdir: str = None) -> str:
    if os.path.isabs(fname):
        return fname

    if fdir is None:
        raise Exception("For relative fname fdir is required")

    return os.path.join(fdir, fname)


def write_json(results: Dict, fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def read_json(fname: str):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "r") as outfile:
        return json.loads(outfile.read())


def read_accuracies_from_results(dname: str):
    """Reads unitary and federated accuracy from results.json
    :param dname: directory in which the results.json file can be found
    """
    fname = os.path.join(FLAGS.results_dir, dname, "results.json")
    json = read_json(fname)

    return (
        json["name"],
        json["unitary_learning"]["acc"],
        json["federated_learning"]["acc"],
    )


def read_uni_vs_fed_acc_stats(filter_substring: str) -> List[Tuple[str, float, float]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    :param filter_substring: has to be part of the dir name in results directory

    :returns: List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    """
    assert os.path.isdir(FLAGS.results_dir)

    # get list of all directories which contain given substring
    matches = list(
        filter(lambda d: filter_substring in d, os.listdir(FLAGS.results_dir))
    )

    assert len(matches) > 0, "No values results found for given group_name"

    return list(map(read_accuracies_from_results, matches))


def plot_uni_vs_fed_acc_stats():
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    values = read_uni_vs_fed_acc_stats(filter_substring=FLAGS.group_name)

    # reverse order data by name
    # e.g. "fashion_mnist_100p_07cpp" before "fashion_mnist_100p_05cpp",
    sorted_values = sorted(values, key=lambda v: v[0], reverse=True)
    indices = range(1, len(sorted_values) + 1)

    # For better understanding:
    # zip(*[('a0', 'b0', 'c0'), ('a1', 'b1', 'c1')]) == [('a0', 'a1'), ('b0', 'b1'), ('c0', 'c1')]
    # list(('a0', 'a1')) == ['a0', 'a1']
    benchmark_names, unitary_accuracies, federated_accuracies = map(
        list, zip(*sorted_values)
    )

    data = [
        ("unitary", unitary_accuracies, indices),
        ("federated", federated_accuracies, indices),
    ]

    fname = plot_iid_noniid_comparison(
        data,
        xticks_args=(indices, [name[19:] for name in benchmark_names]),
        fname=f"plot_{FLAGS.group_name}.png",
    )

    print(f"Data plotted and saved in {fname}")


def plot_iid_noniid_comparison(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    xticks_args: Optional[Tuple[List[int], List[str]]],
    fname="plot.png",
) -> str:
    """
    Plots IID and Non-IID dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    assert len(data) == 2, "Expecting a list of two curves"

    return _plot(
        data,
        title="Max achieved accuracy for unitary and federated learning",
        xlabel="partitioning grade",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=12,
        xticks_args=xticks_args,
        legend_loc="upper right",
    )


def plot_accuracies(
    data: List[Tuple[str, List[float], Optional[List[int]]]], fname="plot.png"
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data])

    return _plot(
        data,
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )


def _plot(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    title: Optional[str] = None,
    xlabel: str = "epoch",
    ylabel: str = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
    xticks_args: Optional[Tuple[List[int], List[str]]] = None,
    legend_loc: str = "lower right",
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)

    :returns: For save=True returns absolut path to saved file otherwise None
    """
    assert fname is not None

    fname_abspath = get_abspath(fname, FLAGS.output_dir)

    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)

    if xticks_args is not None:
        xticks_locations, xticks_labels = xticks_args
        # if any label has length > 3 rotate labels by 90 degrees
        rot = 90 if any([len(l) > 3 for l in xticks_labels]) else 0
        plt.xticks(xticks_locations, xticks_labels, rotation=rot)

    if title is not None:
        plt.title(title)

    plt.xlabel(xlabel)
    plt.ylabel(ylabel)

    legend = []

    for name, values, indices in data:
        legend.append(name)

        if indices is None:
            # x values are optional and default to range(len(values))
            plt.plot(values)
        else:
            assert len(values) == len(indices)
            plt.plot(indices, values)

    plt.legend(legend, loc=legend_loc)

    # https://matplotlib.org/users/tight_layout_guide.html
    plt.tight_layout()

    if save:
        # if fname is an absolute path use fname directly otherwise assume
        # fname is filename and prepend output_dir
        plt.savefig(fname=fname_abspath, format=FORMAT)
    if show:
        plt.show()
    plt.close()

    return fname_abspath


def main(_):
    plot_uni_vs_fed_acc_stats()


if __name__ == "__main__":
    app.run(main=main)
