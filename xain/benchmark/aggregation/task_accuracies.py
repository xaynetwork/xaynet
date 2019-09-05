from typing import List, Optional, Tuple

from absl import flags

from .plot import plot as mpl_plot

FLAGS = flags.FLAGS


def plot(
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

    return mpl_plot(
        data,
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )
