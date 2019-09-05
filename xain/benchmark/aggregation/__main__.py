from absl import app, flags

from .aggregation import aggregate


def app_run_aggregate():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: aggregate())


if __name__ == "__main__":
    app_run_aggregate()
