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


def write_json(results: Dict, fname="results.json"):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def read_json(fname="results.json"):
    fname = get_abspath(fname, FLAGS.output_dir)
    with open(fname, "r") as outfile:
        return json.loads(outfile.read())


def read_uni_vs_fed_acc_stats(
    filter_substring: str
) -> Tuple[Dict[str, float], Dict[str, float]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    :param filter_substring: has to be part of the dir name in results directory

    :returns: tuple of dicts (unitary, federated) which contain Dict[benchmark_name, accuracy]
    """
    assert os.path.isdir(FLAGS.results_dir)

    # get list of all directories which contain given substring
    matches = [
        dname for dname in os.listdir(FLAGS.results_dir) if filter_substring in dname
    ]

    # get list of all results.json files
    result_files = [os.path.join(FLAGS.results_dir, m, "results.json") for m in matches]

    # get list of dicts for all results.json files
    jsons = [read_json(fname) for fname in result_files]

    # Values in the form { benchmark_name: acc } for unitary and federated learning
    uni_values = {json["name"]: json["unitary_learning"]["acc"] for json in jsons}
    fed_values = {json["name"]: json["federated_learning"]["acc"] for json in jsons}

    return (uni_values, fed_values)


def plot_iid_noniid_comparison_from_results():
    uni_values, fed_values = read_uni_vs_fed_acc_stats(
        filter_substring=FLAGS.IID_nonIID_group_name
    )

    order = [
        "fashion_mnist_100p_IID_balanced",
        "fashion_mnist_100p_10cpp",
        "fashion_mnist_100p_09cpp",
        "fashion_mnist_100p_08cpp",
        "fashion_mnist_100p_07cpp",
        "fashion_mnist_100p_06cpp",
        "fashion_mnist_100p_05cpp",
        "fashion_mnist_100p_04cpp",
        "fashion_mnist_100p_03cpp",
        "fashion_mnist_100p_02cpp",
        "fashion_mnist_100p_01cpp",
    ]

    data = [
        ("unitary", [uni_values[n] for n in order], range(1, 12, 1)),
        ("federated", [fed_values[n] for n in order], range(1, 12, 1)),
    ]

    fname = plot_iid_noniid_comparison(data)

    print(f"Data ploted and save in {fname}")


def plot_iid_noniid_comparison(
    data: List[Tuple[str, List[float], Optional[List[int]]]], fname="plot.png"
) -> str:
    """
    Plots IID and Non-IID dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    assert len(data) == 2, "Expecting a list of two curves"

    xticks_locations = list(range(1, 12, 1))
    xticks_labels = ["IID"] + [str(n) for n in range(10, 0, -1)]

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
        xticks_args=(xticks_locations, xticks_labels),
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
        plt.xticks(xticks_locations, xticks_labels)

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

    plt.legend(legend, loc="lower right")

    if save:
        # if fname is an absolute path use fname directly otherwise assume
        # fname is filename and prepend output_dir
        plt.savefig(fname=fname_abspath, format=FORMAT)
    if show:
        plt.show()
    plt.close()

    return fname_abspath


def main(_):
    plot_iid_noniid_comparison_from_results()


if __name__ == "__main__":
    app.run(main=main)
