from autofl.datasets import cifar10_random_splits_10

# Triggering load_splits will fetch datasets from remote and store
# them locally if FETCH_DATASETS=1 is set which is by default set to 0
cifar10_random_splits_10.load_splits()
