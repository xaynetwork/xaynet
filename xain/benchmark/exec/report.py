import os
from typing import List, Optional, Tuple

import matplotlib
from absl import app, flags, logging

from xain.helpers import storage

matplotlib.use("AGG")

# To avoid issues with tkinter we need to set the renderer
# for matplotlib before importing pyplot
# As isort would move this line under the "import matplotlib"
# We need to skip isort explicitly
# pylint: disable-msg=wrong-import-position, wrong-import-order
import matplotlib.pyplot as plt  # isort:skip


FORMAT: str = "png"

FLAGS = flags.FLAGS


def read_accuracies_from_results_file(fname: str) -> Tuple[str, float, float]:
    """Reads unitary and federated accuracy from results.json
    :param dname: directory in which the results.json file can be found
    """
    data = storage.read_json(fname)

    return (
        data["name"],
        data["unitary_learning"]["acc"],
        data["federated_learning"]["acc"],
    )


def read_accuracies_from_group(group_dir: str) -> List[Tuple[str, float, float]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    :param filter_substring: has to be part of the dir name in results directory

    :returns: List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    """
    assert os.path.isdir(group_dir)

    # get list of all directories which contain given substring
    json_files = [
        fname
        for fname in storage.listdir_recursive(group_dir, relpath=False)
        if fname.endswith("results.json")
    ]

    if not json_files:
        raise Exception(f"No values results found in group_dir: {group_dir}")

    # Read accuracies from each file and return list of values in tuples
    return [read_accuracies_from_results_file(fname) for fname in json_files]


PlotValues = Tuple[str, List[float], Optional[List[int]]]
XticksLocations = List[int]
XticksLabels = List[str]


def prepare_iid_noniid_comparison_data(
    group_name: str
) -> Tuple[List[PlotValues], Optional[Tuple[XticksLocations, XticksLabels]]]:
    """Constructs and returns curves and xticks_args

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        (
            [
                ("unitary", unitary_accuracies, indices),
                ("federated", federated_accuracies, indices)
            ],
            (xticks_locations, xticks_labels))
        )
    """
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    values = read_accuracies_from_group(group_name)

    # reverse order data by name
    # e.g. "fashion-mnist-100p-noniid-07cpp" before "fashion-mnist-100p-noniid-05cpp",
    sorted_values = sorted(values, key=lambda v: v[0], reverse=True)
    indices = list(range(1, len(sorted_values) + 1))

    # For better understanding:
    # zip(*[('a0', 'b0', 'c0'), ('a1', 'b1', 'c1')]) == [('a0', 'a1'), ('b0', 'b1'), ('c0', 'c1')]
    # list(('a0', 'a1')) == ['a0', 'a1']
    names, unitary_accuracies, federated_accuracies = [
        list(l) for l in zip(*sorted_values)
    ]

    data: List[PlotValues] = [
        ("unitary", unitary_accuracies, indices),
        ("federated", federated_accuracies, indices),
    ]

    labels: List[str] = [name[19:] for name in names]

    return (data, (indices, labels))


def plot_iid_noniid_comparison() -> str:
    """
    Plots IID and Non-IID dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    fname = f"plot_{group_name}.png"

    (data, xticks_args) = prepare_iid_noniid_comparison_data(group_name)

    assert len(data) == 2, "Expecting a list of two curves"

    fpath = _plot(
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

    logging.info(f"Data plotted and saved in {fname}")

    return fpath


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

    fname_abspath = storage.get_abspath(fname, FLAGS.output_dir)

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
    plot_iid_noniid_comparison()


if __name__ == "__main__":
    flags.mark_flag_as_required("group_name")
    app.run(main=main)
