from absl import flags

from benchmarks.benchmark.net import model_fns

flags.DEFINE_string(
    "task_name",
    None,
    "Mainly used for directory names and as a reference. If label not given used also for plots",
)

flags.DEFINE_string("task_label", None, "Label to be used in plots")

flags.DEFINE_string("model", None, f"Model name, one of {[fn for fn in model_fns]}")

flags.DEFINE_string("dataset", None, "Dataset name")

flags.DEFINE_integer("R", None, "Rounds of federated learning")

flags.DEFINE_integer("E", None, "Epochs of training in each round")

flags.DEFINE_float("C", None, "Fraction of participants participating in each round")

flags.DEFINE_integer("B", None, "Batch size")

flags.DEFINE_integer("partition_id", None, "Partition ID for unitary training")

flags.DEFINE_bool("push_results", True, "Indicates if results should be pushed to S3")
