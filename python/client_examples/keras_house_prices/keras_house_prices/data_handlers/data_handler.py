"""DataHandler base class to read, preprocess and split data for each example."""

from abc import ABC, abstractmethod
import logging
import os
from typing import Dict, List

import numpy as np
import pandas as pd

LOG = logging.getLogger(__name__)


class DataHandler(ABC):  # pylint: disable=too-many-instance-attributes
    """Base class to handle data preparation for all relevant examples
    # TODO: for now implemented only for regression examples, see: AP-144

    Testcase classes inheriting from DataHandler will have to implement
    the abstract methods here, while the rest is automated with DataHandler.

    Args:
         testcase (str): The currently supported testcases. Currently we support:
            'regression', 'image_classification', 'speech_recognition'.
         homogeneity (str): The level of homogeneity in the assignment of
            training samples to each participants. It can take three values:
                'iid': meaning samples are randomely assigned to participants.
                'intermediate': half of the samples are randomely assigned to participants,
                    half of the samples follow the 'total_split' logic.
                'total_split': if there are more participants than labels, samples are split
                    among participants so that each participant has samples from only one class.
                    if there are more classes than participants, samples are split so that
                    no class is repeated between participants.
         n_participants (int): The number of participants into which the dataset will be split.

    NOTE: the random seed is set in the initialisation and will make the results reproducible.
    """

    TEST_RATIO: float = 0.1
    SUPPORTED_VALUES_BY_ATTRIBUTE: Dict[str, List[str]] = {
        "testcase": ["regression", "speech_recognition", "image_classification"],
        "homogeneity": ["iid", "intermediate", "total_split"],
    }
    IS_CLASSIFICATION_BY_TESTCASE: Dict[str, bool] = {
        "regression": False,
        "speech_recognition": True,
        "image_classification": True,
    }
    MINIMUM_PARTICIPANT_N_SAMPLES: int = 20

    def __init__(
        self,
        data_directory: str,
        testcase: str,
        homogeneity: str = "iid",
        n_participants: int = 10,
    ) -> None:
        self.testcase: str = self.check_and_return_if_valid(testcase, "testcase")
        self.homogeneity: str = self.check_and_return_if_valid(
            homogeneity, "homogeneity"
        )
        self.n_participants: int = n_participants
        self.participant_ids: List[str] = [str(p) for p in range(self.n_participants)]
        # TODO: once AP-86 (automatic download) is done, the path will be obvious
        #  for now, the path below is 'xain-benchmark/xain_benchmark/data/{testcase}}'
        self.data_dir: str = data_directory
        self.parts_dir: str = os.path.join(self.data_dir, "split_data")
        if not os.path.exists(self.parts_dir):
            os.mkdir(self.parts_dir)
            LOG.info("created {} dir".format(self.parts_dir))
        self.train_file_path: str = os.path.join(self.data_dir, "train.csv")
        self.test_file_path: str = os.path.join(self.data_dir, "test.csv")
        self.train_df: pd.DataFrame = pd.DataFrame()
        self.test_df: pd.DataFrame = pd.DataFrame()
        self.labels: List[str] = []

        # set the seed that will be used by numpy to make the results reproducible.
        np.random.seed(42)

    def check_and_return_if_valid(self, input_value: str, attribute: str) -> str:
        """Check that the args with which the class has been initialised are supported.

        Args:
            input_value (str): The value of the input attribute to check.
            attribute (str): The name of the attribute to check.

        Returns:
            input_value (str): The value of the attribute, if supported.

        Raises:
            ValueError: if the input_value is not among the supported values for that attribute.
        """

        if input_value not in self.SUPPORTED_VALUES_BY_ATTRIBUTE[attribute]:
            message = "{} is not currently supported. supported {}s: {}".format(
                input_value, attribute, self.SUPPORTED_VALUES_BY_ATTRIBUTE[attribute]
            )
            raise ValueError(message)
        return input_value

    @abstractmethod
    def download_data(self):
        """Abstract method to be implemented by the testcase data handling subclass,
        to download the data from its source.
        """

        raise NotImplementedError()

    @abstractmethod
    def read_data(self):
        """Abstract method to be implemented by the testcase data handling subclass,
        to read the data from the path where it has been saved.
        """

        raise NotImplementedError()

    @abstractmethod
    def preprocess_data(self):
        """Abstract method to be implemented by the testcase data handling subclass,
        to preprocess the data, which is specific to each testcase.
        """

        raise NotImplementedError()

    def create_testset(self) -> None:
        """Create testset by sampling and removing a TEST_RATIO percentage of samples
        from self.train_df. Save the data locally.
        """

        n_test_samples: int = int(len(self.train_df) * self.TEST_RATIO)
        test_indexes: np.ndarray = np.random.choice(
            self.train_df.index, n_test_samples, replace=False
        )
        self.test_df: pd.DataFrame = self.train_df.loc[test_indexes, :]
        self.train_df: pd.DataFrame = self.train_df.drop(test_indexes)
        self.test_df.to_csv(self.test_file_path)

    def make_discrete_y_if_continuous(self) -> pd.Series:
        """Split a continuous Y variable into discrete bins, one per participant,
        or returns it, if already discrete.

        Returns:
            discrete_y (pd.Series): The discrete dependent variable.
        """

        is_classification: bool = self.IS_CLASSIFICATION_BY_TESTCASE[self.testcase]
        if is_classification:
            discrete_y: pd.Series = self.train_df["Y"]
        else:
            discrete_y: pd.Series = pd.cut(
                self.train_df["Y"],
                bins=self.n_participants,
                labels=range(self.n_participants),
            )

        self.labels = list(set(discrete_y))
        return discrete_y

    def make_iid_split(
        self,
        input_df: pd.DataFrame,
        target_length: int,
        assigned_samples: List[str] = None,
    ) -> np.ndarray:
        """Randomly select samples so that each participant has a similar amount of samples.

        Args:
            input_df (pd.DataFrame): DataFrame containing the samples to be selected.
            target_length (int): Length of the full dataset considered for IID split.
            assigned_samples (List[str]): Optional. List of sample IDs already assigned
                to previous participants.

        Returns:
            selected_sample_ids (np.ndarray): The selected sample indexes.
        """

        if assigned_samples is not None:
            input_df: pd.DataFrame = input_df.drop(assigned_samples)
        samples_ids_per_participant: int = int(target_length / self.n_participants)
        selected_sample_ids: np.ndarray = np.random.choice(
            input_df.index, samples_ids_per_participant, replace=False
        )
        return selected_sample_ids

    @staticmethod
    def split_lists(
        longer_list: List[str], shorter_list: List[str]
    ) -> Dict[str, List[str]]:
        """Split the lists of labels and participant IDs.

        We use longer and shorter list to make sure that the elements of the longer list
        are distributed to the elements of the shorter.

        For example:
        - If there are more participants than labels, the samples of each label will be
        distributed to different participants, and each participant will have samples
        from only one label.
        - If there are more labels than participants, each participant will have samples
        from more than one label, but samples from a single label will belong to only one
        participant.

        Args:
            longer_list (List[str]): List of either labels or participant IDS,
                whichever is longer.
            shorter_list (List[str]): List of either labels or participant IDS,
                whichever is shorter.

        Returns:
            splits_by_shorter_element (Dict[str, List[str]]):
                Dictionary  whose keys are the elements of the shorted list,
                and its values are a sample without replacement of the elements
                of the longer list.
        """

        ratio: int = len(longer_list) // len(shorter_list)
        splits: List[List[str]] = [
            longer_list[i : i + ratio] for i in range(0, len(longer_list), ratio)
        ]
        splits_by_shorter_element: Dict[str, List[str]] = {
            item: splits[i] for i, item in enumerate(shorter_list)
        }
        return splits_by_shorter_element

    def make_total_split(
        self, discrete_y: pd.Series, participant_id: str, participant_ids: List[str]
    ) -> np.ndarray:
        """Select labels for one participant.

        If there are more labels than participants, it will select a list of labels not
        assigned to any other participant. If there are more participants than labels,
        it will select one label only for this participant
        (the label may re-occur for other participants).

        Args:
            discrete_y (pd.Series): The discrete dependent variable.
            participant_id (str): The ID of the participant for which we are currently selecting
                the samples for its dataset.
            participant_ids (List[str]): List of all participant IDs.

        Returns:
            selected_samples (np.ndarray): List of selected samples for the current participant.
        """

        if len(self.labels) >= self.n_participants:
            labels_by_participant_id: Dict[str, List[str]] = self.split_lists(
                list(self.labels), participant_ids
            )
            selected_labels: List[str] = labels_by_participant_id[participant_id]
        else:
            participant_ids_by_label: Dict[str, List[str]] = self.split_lists(
                participant_ids, self.labels
            )
            selected_labels: List[str] = [
                label
                for label, ids in participant_ids_by_label.items()
                if participant_id in ids
            ]
        selected_samples: np.ndarray = np.array(
            [i for i, label in discrete_y.items() if label in selected_labels]
        )
        return selected_samples

    def make_intermediate_split(
        self, assigned_samples: List[str], participant_id: str, discrete_y: pd.Series
    ) -> np.ndarray:
        """Handles an intermediate split, 50% IID and 50% total_split.

        Args:
            assigned_samples (List[str]): Samples that have already been assigned to a participant.
            participant_id (str): The ID of the participant that will have samples assigned to.
            discrete_y (pd.Series): The discrete dependent variable.

        Raises:
            AssertionError: If the selected samples are not unique.
            Typically if there was replacement, or the random seed had not been set.

        Returns:
            selected_samples (np.ndarray): The IDs of the selected samples for this participant.
        """

        remaining_samples_df: pd.DataFrame = self.train_df.drop(assigned_samples)
        first_half_df: pd.DataFrame = remaining_samples_df.sample(frac=0.5)
        second_half_df: pd.DataFrame = remaining_samples_df.drop(first_half_df.index)
        target_length: int = len(self.train_df) // 2
        iid_samples: np.ndarray = self.make_iid_split(first_half_df, target_length)
        second_half_y: pd.Series = discrete_y.loc[second_half_df.index]
        total_split_samples: np.ndarray = self.make_total_split(
            second_half_y, participant_id, self.participant_ids
        )
        selected_samples: np.ndarray = np.concatenate(
            (iid_samples, total_split_samples)
        )
        if len(set(selected_samples)) != len(selected_samples):
            raise AssertionError
        return selected_samples

    def split_data(self) -> None:
        """Split the data.

        Continuous variables (for regression) are made discrete only for the purpose of
        splitting the data (not for analysis).

        For each participant ID, it performs the data split according to the level
        of homogeneity selected.

        Saves the dataframe for each participant locally.
        """

        discrete_y: pd.Series = self.make_discrete_y_if_continuous()
        np.random.shuffle(self.labels)
        np.random.shuffle(self.participant_ids)
        assigned_samples: List[str] = []
        for participant_id in self.participant_ids:
            if self.homogeneity == "iid":
                selected_samples: np.ndarray = self.make_iid_split(
                    self.train_df, len(self.train_df), assigned_samples
                )
            elif self.homogeneity == "total_split":
                selected_samples: np.ndarray = self.make_total_split(
                    discrete_y, participant_id, self.participant_ids
                )
            else:
                selected_samples: np.ndarray = self.make_intermediate_split(
                    assigned_samples, participant_id, discrete_y
                )
            participant_df: pd.DataFrame = self.train_df.loc[selected_samples, :]
            LOG.info(
                "participant {} df has shape {}".format(
                    participant_id, participant_df.shape
                )
            )
            if len(participant_df) < self.MINIMUM_PARTICIPANT_N_SAMPLES:
                LOG.info(
                    "participant {} has only {} samples.".format(
                        participant_id, len(participant_df)
                    )
                )
                LOG.info("consider decreasing the number of participants")
                # TODO: edge case: non-IID splits (especially 'total_split') with
                #  too many participants may lead to an empty df. Pandas will save
                #  the CSV anyway, but we may have problems reading the files later.
                #  Solve this with: https://xainag.atlassian.net/browse/AP-154
            output_filepath: str = os.path.join(
                self.parts_dir, f"data_part_{participant_id}.csv"
            )
            participant_df.to_csv(output_filepath, index=False)
            LOG.info("participant df saved to {}".format(output_filepath))
            assigned_samples.extend(participant_df.index)

    def run(self) -> None:
        """One function to run them all.
        """

        self.download_data()
        self.read_data()
        self.preprocess_data()
        self.create_testset()
        self.split_data()
