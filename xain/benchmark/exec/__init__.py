from absl import flags

from xain.benchmark.net import model_fns

flags.DEFINE_string("task_name", None, "")

flags.DEFINE_string("model", None, f"Model name, one of {[fn for fn in model_fns]}")

flags.DEFINE_string("dataset", None, "Dataset name")

flags.DEFINE_integer("R", None, "Rounds of federated learning")

flags.DEFINE_integer("E", None, "Epochs of training in each round")

flags.DEFINE_float("C", None, "Fraction of participants participating in each round")

flags.DEFINE_integer("B", None, "Batch size")
