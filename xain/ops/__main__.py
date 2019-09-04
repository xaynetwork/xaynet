from absl import app

from . import results


def download():
    app.run(main=lambda _: results.download())
