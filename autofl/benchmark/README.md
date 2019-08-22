# Benchmarks

This package contains all the benchmarks to be run.

## Plots

Generally plots will be automatically generated when a benchmark is triggered.

**Exception:**<br>
The plot which compares the top performances of unitary and federated learning on the IID to Non-IID datasets aggregates top accuracies of a given benchmark group in the results directory and plots them.
To do this you have to first run a group of benchmarks with e.g. the `train_remote_iid_noniid.sh` script in scripts. Afterwards you can plot the values using

```bash
python -m autofl.benchmark.report --group_name GROUP_NAME
```

When executing `train_remote_iid_noniid.sh` you will be asked for a GROUP_NAME
Alternatively when running `train_remote.sh` you can set it via the ENV variable `BENCHMARK_GROUP`
