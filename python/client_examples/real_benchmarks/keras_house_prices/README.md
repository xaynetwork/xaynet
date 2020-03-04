# Regression tutorial for Xain Federated Learning module

In this example we explore setting and running a regression example
based on the Advanced House Price Prediction problem. We show how to
download, split and set up the dataset for this problem, how to build
a predictive model and how to integrate it with XAIN participant data.

## Downloading and preparing the data

House Prices: Advanced Regression Techniques is a problem available on
[Kaggle
platform](https://www.kaggle.com/c/house-prices-advanced-regression-techniques/data). It
consists of a list of features of houses sold in the Boston area
together with the sale prices. The task is to predict future prices of
houses based on the features. Our first task is to heal, engineer and
scale the data which then is split into parts used by participants.

All the data processing is done in the `regression_data.py` (which can
be found in `xain_benchmark/data_handlers/`) file that performs the
data preparation, creates a testset and splits the data for each
participant.

The class attributes are:

- `framework`: `keras` in this case.
- `homogeneity`: it can take three values:
    - `iid`: the samples are randomly distributed across the participants
    - `total_split`: the samples are distributed so that there's no
      class overlap among participants
    - `intermediate`: half of the samples are randomly distributed,
      the other half follows the "total_split" logic

After downloading and unpacking the data, run:

```
prepare-regression-data --data-directory data/ --number-of-participants 100
```


## Regression model with Tensorflow Keras

To predict house price we create a Regressor model with a simple
architecture of 4 dense layers, MSE loss function and Adam optimizer.

```python
def __init__(self, dim: int):
    self.model = Sequential()
    self.model.add(Dense(144, input_dim=dim, activation="relu"))
    self.model.add(Dense(72, activation="relu"))
    self.model.add(Dense(18, activation="relu"))
    self.model.add(Dense(1, activation="linear"))

    self.model.compile(optimizer="adam", loss="mean_squared_error")
```

as this model can be used for more generic regression problem we leave
the input data shape as a parameter.


## XAIN Participant

Within the XAIN Participant class we first randomly select a subsample
of the data we created during the split.  Then, we input a testset
that is used for validation purposes. The main part of the Participant
is the model that is an instance of the Regressor class containing 4
fully connected layers.

To ensure random input from the user we use the random sample of the
dataset which is then split into labels and features (or x and y
variables).

```python
trainset = pd.read_csv(trainset_file_path, index_col=None)
testset = pd.read_csv(testset_file_path, index_col=None)
self.trainset_x = trainset.drop("Y", axis=1)
self.trainset_y = trainset["Y"]
self.testset_x = testset.drop("Y", axis=1)
self.testset_y = testset["Y"]
self.model = Regressor(len(self.trainset_x.columns))
self.shapes = self.get_tensorflow_shapes(model=self.model.model)
self.flattened = self.get_tensorflow_weights(model=self.model.model)
self.number_samples = len(trainset)
```


## Training round

Training round contains of 4 parts implemented in the `train_round()`
method:

- Inputting the weights in the regressor model
- Training round with given number of epochs
- Flattening the model and sending it back to the coordinator
- Calculating the loss on the test set created during the split, sending the metrics to coordinator

If there are no weights provided, then the participant initializes new
weights according to its model definition and returns them without
further training as implemented in the `init_weights()` method.
