from absl import app, logging

from xain.datasets import testing
from xain.generator import config, data, persistence


def generate_dataset(dataset_name):
    logging.info("Starting dataset generation of {}".format(dataset_name))

    assert dataset_name in config.datasets, "Dataset not found in config"

    c = config.datasets[dataset_name]

    dataset = data.create_federated_dataset(
        keras_dataset=c["keras_dataset"],
        num_partitions=c["num_partitions"],
        validation_set_size=c["validation_set_size"],
        transformers=c["transformers"],
        transformers_kwargs=c["transformers_kwargs"],
    )

    if c["assert_dataset_origin"]:
        testing.assert_dataset_origin(
            keras_dataset=data.load(c["keras_dataset"]), federated_dataset=dataset
        )

    persistence.save_splits(
        dataset_name=dataset_name,
        dataset=dataset,
        local_generator_dir=config.local_generator_datasets_dir,
    )


def main(_):
    for dataset_name in config.datasets:
        generate_dataset(dataset_name)


app.run(main=main)
