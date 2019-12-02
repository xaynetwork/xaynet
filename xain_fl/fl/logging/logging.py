"""Functions for logging TensorBoard summaries"""

import tensorflow as tf
from absl import flags
from tensorflow._api.v1.compat.v1 import Summary
from tensorflow._api.v1.compat.v1.summary import FileWriter

FLAGS = flags.FLAGS


def create_summary_writer(logdir: str) -> FileWriter:
    """Creating a summary FileWriter.

    Creates a FileWriter that create an event file in a given directory and add summaries and
    events to it. The file contents are updated asynchronously.

    Args:
        logdir (str): Directory in which the tensorboard evnt log is written.

    Returns:
        ~tf.summary.FileWriter: FileWriter object writing Summaries to event files.
    """

    summary_writer = FileWriter(logdir=logdir, graph=tf.compat.v1.get_default_graph())
    return summary_writer


def write_summaries(
    summary_writer: FileWriter, val_acc: float, val_loss: float, train_round: int
) -> None:
    """Adding summaries to an event file.

    Adds validation loss and accuracy as scalar values, as well as the global step value to the
    event file.

    Args:
        summary_writer (~tf.summary.FileWriter): FileWriter object writing Summaries to event files.
        val_acc (float): Validation accuracy that should be logged to event file.
        val_loss (float): Validation loss that should be logged to event file.
        train_round (int): Train round, that should be logged as dependency for acc and loss.
    """

    summary_writer.add_summary(
        summary=Summary(
            value=[
                Summary.Value(
                    tag=f"coordinator_{FLAGS.task_name}/val_acc", simple_value=val_acc
                )
            ]
        ),
        global_step=train_round,
    )

    summary_writer.add_summary(
        summary=Summary(
            value=[
                Summary.Value(
                    tag=f"coordinator_{FLAGS.task_name}/val_loss", simple_value=val_loss
                )
            ]
        ),
        global_step=train_round,
    )
    # flushing each training round to observe live training in TensorBoard dashboard
    summary_writer.flush()
