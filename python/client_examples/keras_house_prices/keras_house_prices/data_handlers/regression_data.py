"""Implementation of the RegressionData subclass, to handle the data of regression examples."""

import argparse
import logging

import numpy as np
import pandas as pd
from sklearn.preprocessing import MinMaxScaler

from keras_house_prices.data_handlers.data_handler import DataHandler

LOG = logging.getLogger(__name__)


class RegressionData(DataHandler):
    """Data processing logic that is specific to the house prices dataset.

    """

    def __init__(
        self, data_directory: str, homogeneity: str, n_participants: int
    ) -> None:
        super(RegressionData, self).__init__(
            data_directory, homogeneity=homogeneity, n_participants=n_participants
        )

    def fill_nan(self) -> None:
        """Filling missing data in the dataframe."""

        self.train_df["PoolQC"] = self.train_df["PoolQC"].fillna("None")
        self.train_df["MiscFeature"] = self.train_df["MiscFeature"].fillna("None")
        self.train_df["Alley"] = self.train_df["Alley"].fillna("None")
        self.train_df["Fence"] = self.train_df["Fence"].fillna("None")
        self.train_df["FireplaceQu"] = self.train_df["FireplaceQu"].fillna("None")
        self.train_df["LotFrontage"] = self.train_df.groupby("Neighborhood")[
            "LotFrontage"
        ].transform(lambda x: x.fillna(x.median()))
        for col in ("GarageType", "GarageFinish", "GarageQual", "GarageCond"):
            self.train_df[col] = self.train_df[col].fillna("None")
        for col in ("GarageYrBlt", "GarageArea", "GarageCars"):
            self.train_df[col] = self.train_df[col].fillna(0)
        for col in (
            "BsmtFinSF1",
            "BsmtFinSF2",
            "BsmtUnfSF",
            "TotalBsmtSF",
            "BsmtFullBath",
            "BsmtHalfBath",
        ):
            self.train_df[col] = self.train_df[col].fillna(0)
        for col in (
            "BsmtQual",
            "BsmtCond",
            "BsmtExposure",
            "BsmtFinType1",
            "BsmtFinType2",
        ):
            self.train_df[col] = self.train_df[col].fillna("None")
        self.train_df["MSZoning"] = self.train_df["MSZoning"].fillna(
            self.train_df["MSZoning"].mode()[0]
        )

        self.train_df["MasVnrType"] = self.train_df["MasVnrType"].fillna("None")
        self.train_df["MasVnrArea"] = self.train_df["MasVnrArea"].fillna(0)
        self.train_df = self.train_df.drop(["Utilities"], axis=1)
        self.train_df["Functional"] = self.train_df["Functional"].fillna("Typ")
        self.train_df["Electrical"] = self.train_df["Electrical"].fillna(
            self.train_df["Electrical"].mode()[0]
        )
        self.train_df["KitchenQual"] = self.train_df["KitchenQual"].fillna(
            self.train_df["KitchenQual"].mode()[0]
        )
        self.train_df["Exterior1st"] = self.train_df["Exterior1st"].fillna(
            self.train_df["Exterior1st"].mode()[0]
        )
        self.train_df["Exterior2nd"] = self.train_df["Exterior2nd"].fillna(
            self.train_df["Exterior2nd"].mode()[0]
        )
        self.train_df["SaleType"] = self.train_df["SaleType"].fillna(
            self.train_df["SaleType"].mode()[0]
        )
        self.train_df["MSSubClass"] = self.train_df["MSSubClass"].fillna("None")

        no_nulls_in_dataset = not self.train_df.isnull().values.any()
        if no_nulls_in_dataset:
            LOG.info("No missing values")
            LOG.info("data shape is %s", self.train_df.shape)

    def hot_encoding(self) -> None:
        """Hot encoding of the categorical features."""

        self.train_df: pd.DataFrame = pd.get_dummies(
            self.train_df, dummy_na=True, drop_first=True
        )
        LOG.info("data shape is %s", self.train_df.shape)

    def scaling(self) -> None:
        """Scales the features in minmax way and the process in log(1+x)."""

        self.train_df = self.train_df.rename(columns={"SalePrice": "Y"})
        self.train_df["Y"] = np.log1p(self.train_df["Y"])
        scaler = MinMaxScaler()
        cols = self.train_df.drop("Y", axis=1).columns
        train = pd.DataFrame(
            scaler.fit_transform(self.train_df.drop("Y", axis=1)), columns=cols
        )
        self.train_df[cols] = train

    def preprocess_data(self) -> None:
        """Call methods that execute the preprocessing.

        """
        self.train_df.drop("Id", axis=1, inplace=True)
        self.fill_nan()
        self.hot_encoding()
        self.scaling()


def main() -> None:
    """Initialise and run the regression data preparation."""
    logging.basicConfig(level=logging.DEBUG)

    parser = argparse.ArgumentParser(description="Prepare data for regression")
    parser.add_argument(
        "--data-directory",
        type=str,
        help="path to the directory that contains the raw data",
    )
    parser.add_argument(
        "--number-of-participants",
        type=int,
        help="number of participants into which the dataset will be split",
    )
    args = parser.parse_args()

    regression_data = RegressionData(
        args.data_directory, "total_split", args.number_of_participants,
    )
    regression_data.run()
