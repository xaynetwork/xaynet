from absl import app, flags

from . import agent

flags.DEFINE_spaceseplist(
    "arch",
    None,
    "Space-separated list of integers defining the network architecture to use",
)
flags.DEFINE_bool("sample_random_arch", False, "Use a randomly sampled architecture")

# See: https://stackoverflow.com/questions/33703624/how-does-tf-app-run-work
app.run(main=agent.main)
