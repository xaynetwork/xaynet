from absl import flags

from xain.benchmark.net import model_fns

flags.DEFINE_string("model", None, f"Model name, one of {[fn for fn in model_fns]}")
flags.DEFINE_string("dataset", None, "Dataset name")
flags.DEFINE_integer("B", None, "Batch size")
flags.DEFINE_integer("partition_id", None, "Partition ID for unitary training")
