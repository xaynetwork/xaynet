"""Wrapper for tensorflow regression neural network."""
from typing import Tuple

import numpy as np
import pandas as pd
from sklearn.metrics import r2_score
from tensorflow.keras import Sequential  # pylint: disable=import-error
from tensorflow.keras.layers import Dense  # pylint: disable=import-error


class Regressor:
    """Neural network class for the Boston pricing house problem.

    Attributes:
        model: Keras Sequential model
    """

    def __init__(self, dim: int):
        self.model = Sequential()
        self.model.add(Dense(144, input_dim=dim, activation="relu"))
        self.model.add(Dense(72, activation="relu"))
        self.model.add(Dense(18, activation="relu"))
        self.model.add(Dense(1, activation="linear"))

        self.model.compile(optimizer="adam", loss="mean_squared_error")

    def train_n_epochs(
        self, n_epochs: int, x_train: pd.DataFrame, y_train: pd.DataFrame
    ) -> None:
        """Training function for the built in model.

        Args:
            n_epochs (int): Number of epochs to be trained.
            x_train (~pd.dataframe): Features dataset for training.
            y_train(~pd.dataframe): Labels for training.
        """

        self.model.fit(x_train, y_train, epochs=n_epochs, verbose=0)

    def evaluate_on_test(
        self, x_test: pd.DataFrame, y_test: pd.DataFrame
    ) -> Tuple[float, float]:
        """Evaluating on testset.

        Args:
             x_test (dataframe): Feature set for evaluation.
             y_test (dataframe): Dependent variable for evaluation.

        Returns:
            test_loss: Value of the testing loss.
            r_squared: Value of R-squared,
                to be shown as 'accuracy' metric to the Coordinator
         """

        y_pred: np.ndarray = self.model.predict(x_test)
        r_squared: float = r2_score(y_test, y_pred)
        test_loss: float = self.model.evaluate(x_test, y_test)
        return test_loss, r_squared
