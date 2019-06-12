from autofl.data.stats import basic_statistics


def test_basic_statistics_with_default_mnist(dataset_mnist):
    (x_train, y_train, x_test, y_test) = dataset_mnist

    # Making sure our dataset is correct
    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]

    stats = basic_statistics(dataset_mnist)

    assert isinstance(stats, dict)
    assert isinstance(stats["train"], dict)
    assert isinstance(stats["test"], dict)

    assert stats["train"]["number_of_examples"] == 60000

    assert len(stats["train"]["number_of_examples_per_label"][0]) == 10

    print(stats["train"]["number_of_examples_per_label"])

    for count in stats["train"]["number_of_examples_per_label"][1]:
        # not all labels are euqally distributed but definitly more than 5k times in cifar10
        assert count > 5000


def test_basic_statistics_with_default_cifar10(dataset_cifar10):
    (x_train, y_train, x_test, y_test) = dataset_cifar10

    # Making sure our dataset is correct
    assert x_train.shape[0] == y_train.shape[0]
    assert x_test.shape[0] == y_test.shape[0]

    stats = basic_statistics(dataset_cifar10)

    assert isinstance(stats, dict)
    assert isinstance(stats["train"], dict)
    assert isinstance(stats["test"], dict)

    assert stats["train"]["number_of_examples"] == 50000

    assert len(stats["train"]["number_of_examples_per_label"][0]) == 10

    print(stats["train"]["number_of_examples_per_label"])

    for count in stats["train"]["number_of_examples_per_label"][1]:
        # each label will ocur 5k times in cifar10
        assert count == 5000
