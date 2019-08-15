from absl import app, logging

from autofl.datasets import testing
from autofl.generator import config, data, persistence


def generate_dataset(dataset_name):
    logging.info("Starting dataset generation of {}".format(dataset_name))

    assert dataset_name in config.datasets, "Dataset not found in config"

    c = config.datasets[dataset_name]

    dataset = data.generate_splits(
        num_splits=c["num_splits"],
        validation_set_size=c["validation_set_size"],
        keras_dataset=c["keras_dataset"],
        transformer=c["transformer"],
        transformer_kwargs=c["transformer_kwargs"],
    )

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
