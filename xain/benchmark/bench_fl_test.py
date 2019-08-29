import os

import pytest
from absl import flags

from . import bench_fl, run

FLAGS = flags.FLAGS


@pytest.mark.slow
@pytest.mark.integration
def test_run_unitary_versus_federated(output_dir):
    # Prepare
    benchmark_name = "integration_test"
    kwargs = bench_fl.benchmarks[benchmark_name]

    # Execute
    run.unitary_versus_federated(
        benchmark_name=benchmark_name, model_name="blog_cnn", **kwargs
    )

    # Assert
    # check if the files exist as the training is not deterministic
    # and hasing the plot or results does not work
    assert os.path.isfile(os.path.join(output_dir, "plot.png"))
    assert os.path.isfile(os.path.join(output_dir, "results.json"))
