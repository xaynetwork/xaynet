from typing import List

from .results import TaskResult


def test_get_learning_rates(results_json_fname):
    # Prepare
    result = TaskResult(results_json_fname)

    expected_lr_round_1 = 0.1
    expected_lr_round_2 = 0.2

    # Execute
    learning_rates: List[float] = result.get_learning_rates()

    # Assert
    assert isinstance(learning_rates, list)
    assert len(learning_rates) == 2

    assert learning_rates[0] == expected_lr_round_1
    assert learning_rates[1] == expected_lr_round_2
