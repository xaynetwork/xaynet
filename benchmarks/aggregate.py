"""Used as entry_point for console_script `aggregate` in `setup.py`.

Expects the following flags:

    - group_name

Example:
    aggregate --group_name=GROUP_NAME
"""
from absl import app, flags

from benchmarks.benchmark.aggregation import aggregation

FLAGS = flags.FLAGS


def main():
    flags.mark_flag_as_required("group_name")
    app.run(main=aggregation.main)


if __name__ == "__main__":
    main()
