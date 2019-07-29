from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_100p_IID_balanced"
DATASET_SPLIT_HASHES = {
    "00": [
        "4a2b69e7e7947e235e1e3aa86d358b61308f98a6",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "01": [
        "b90240d1c862eaff90d0db28b88b081c163b90bd",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "02": [
        "286aaf3fead74730f6df011b5465013b57b69374",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "03": [
        "cb5a3e6aa6faeb2373458874a62d068e97b81625",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "04": [
        "c72fe12974e620bc4507f8cd8502f8150816a400",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "05": [
        "17d3ca8e46aedb547c27da2d945a3d58004a290c",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "06": [
        "7cbbd1d7a2d40c1ae4a9ecd8d2d74498a60e5285",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "07": [
        "3f7a7513701ab2b579885c0c5eb6e248ca3c7c87",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "08": [
        "cc821b6f9f2aa0d14dde1f6261ea67f614425851",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "09": [
        "b3feba0c40a20fa70a02e6f6dc1ef769dfbe25cf",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "10": [
        "1ffba3ba1660c378ec4b93b4cd84d18d8b3299bf",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "11": [
        "a836cc8c1f716a1cf5f6647afacf6df7f0445da6",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "12": [
        "f98fdda1cc5f78440dfb4821334a94f6151fbb2f",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "13": [
        "b9143458227e262d63a0dfa60a504e924f875e95",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "14": [
        "ab2a965ec1c1912a331b0ac017705983aba2ba37",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "15": [
        "14ae1003aba31fc28e66adc06bb5086dda9382e0",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "16": [
        "590f45b13e2493696af63986bef5d91b89652f52",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "17": [
        "d586c32a432da36c8ea282a7d41646c4d510590b",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "18": [
        "f54f690a0b5be3b967513f2a7dff1f78808e5f9b",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "19": [
        "5fc53bb6fbc8c26b396ce1caec9a65ecb0a52cdc",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "20": [
        "8f436f2e6eb477bd5ba95bf7071c27beb8bdcfea",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "21": [
        "e395716c89e9b491792ac70f6096b685b9c47c99",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "22": [
        "8268a4ff0ef5bc7e622b92844901e713f0ef215e",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "23": [
        "754a93de1aa037cdb5ab0b5fbb81c0785f500e82",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "24": [
        "cd5295ffa20d505074925fe7ce9722263db4bc07",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "25": [
        "d842a2c4e9c3e9537346123b181601719929668a",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "26": [
        "5e919cb8606a90f464ee5d281b5084d480bd53bb",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "27": [
        "fc9572b7cf4f7d522ef3e95002e5718d15d71647",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "28": [
        "ed7e8e6c1cec5a1edc2d17d5f9591e7af1a26b2d",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "29": [
        "8462d2b5336ac5a6fee488f0b0ee21e2baaee985",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "30": [
        "c2d3bf19585e079f1177ea8ac882374313301862",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "31": [
        "3ca04fe54a71d9404de2c7c9747670a7c2816448",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "32": [
        "094e6f4644398281e72fbcf39496e83067c305a2",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "33": [
        "1edbe95dc968d385d224d84b8b7228e1863d2827",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "34": [
        "00ba9859ee8b408ffc29abe12cc37bd9ac4936d7",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "35": [
        "f688b5b185b6c9547dbed055d77e66683932fe76",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "36": [
        "5f2458f18e944bbb9e7ea69acd6b24a80c31e6d9",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "37": [
        "77e7a7a07a2d4a21b1b38955a56b120bebe4e5ab",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "38": [
        "65319b97f998430ad35fd13746162d22eedd3026",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "39": [
        "469529380ffafd5efc71d22c7dc8cdcbbd72782e",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "40": [
        "ba71d9906343a4d660bfa2bd68d96a22cb343531",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "41": [
        "374c3a151b566aeb98ec42b4f48619c900133d18",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "42": [
        "b5ce8997f7dad36e692062b80b91093b81c9a4c6",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "43": [
        "417ffba7aa4760b0bf171c2bc68f6a1dfe3c4bac",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "44": [
        "8f297282bcc41179ecfa003d665de789990a6a7f",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "45": [
        "52ec85506afb1bd7d64277e8caec21476ab1116f",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "46": [
        "541bbbb782efb2236c8ce8fa8921211f5ba50c64",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "47": [
        "e5d9c221f528194a4d3da528a41140af5b8c8896",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "48": [
        "0e9dffb7bdac00b2952db402f123781ec12001e8",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "49": [
        "2afec9ec8e21c9289abc56ed2f21895d68c72ded",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "50": [
        "016b158a390c80c3eaa4e17ffe2c768729a94ddf",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "51": [
        "3ce71177065dde545d2a5e8b2ef2d06e58146919",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "52": [
        "169f0bc8a26d22cab404e48ef113223d22e48acf",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "53": [
        "f863bc5a76d931ef85341709537ffaa708586e6d",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "54": [
        "752cc04a38efdbe24a6a8bdb694bdb42f54a2b53",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "55": [
        "200f901011b0bf976ed90dc46435365f9334aa78",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "56": [
        "ca24473b3537297e1156f17839a9c22a7224de3b",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "57": [
        "e1a3a018561cfb0145bafb179b47ec80895123a6",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "58": [
        "79b66e46cb35c2c01394cb199889f8b176c6d4bf",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "59": [
        "ef00738e8e203c6477e745643960de9ec7f95914",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "60": [
        "8d14803208badb0d2646391ae0148577e14c46e0",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "61": [
        "09277b5dde5b28ebe8276e44737d3f24c257d922",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "62": [
        "151b57b6072ce46aa9127428004af6a94a928ea4",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "63": [
        "56bf51c23342566c2359c46694d3e3f4fb953e2f",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "64": [
        "5f1aaf2686d1c233515e352ae3f64f254d0db048",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "65": [
        "6edb9577e8e9f8031020356482b66de82f9deb73",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "66": [
        "36f01063c3c75e611b52984a63e236d2e584ae24",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "67": [
        "8a699c15ae4479e8516d7a7abe160af020a31a0a",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "68": [
        "105d6715635be5d08192cd8b1eeb602b954f5696",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "69": [
        "b051fcd95f82c039496b814e5253a2cf50ef3ecf",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "70": [
        "2f6c0bc684870678f6e9aab866620ff1bb7f9735",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "71": [
        "c5fb1a904ca8bcb33d64e2beac6dcf2e6d835e83",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "72": [
        "ae1ba37b73f4ad310a220ddfca094f3f9591aa12",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "73": [
        "5088de7188cd42c63e3b84692eaf8ed296c588a6",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "74": [
        "e1cee8781c1f8497a03554efc784fd221a7a6486",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "75": [
        "2733a61c4303b05e4605c2156dc8e81f38d30e7c",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "76": [
        "aef53f248c6de0b62de4142b0952dfb8b2edbbbf",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "77": [
        "b8291dd5bd9f15fbcc1f753c2f15284e267b61d0",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "78": [
        "550a34e4768bae4dc897be379d96bd4f007d25e9",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "79": [
        "c0f371f896c71054037a4107926bb8a1e68c2d1f",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "80": [
        "9c9fecfeff5a198e7557c487d76992d6eeba08d7",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "81": [
        "23a648e097cd415b57d277d1f9d6c9d0ddfcfadf",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "82": [
        "0ef85ab588ae5c74bd88acec62cd59498a284415",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "83": [
        "74869f404ea2e7e18ff0188289b41e620f1ff1c7",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "84": [
        "881e590704d2d92358326dbc6861a74a6892fd79",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "85": [
        "4dde7dd1f0c5245ae111f3b3081609d0be04597b",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "86": [
        "7da14e86491903076c5eb32b0388aa21b79f3b58",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "87": [
        "361fb734b7f7c64b8bcf6cb75381219ae69559fc",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "88": [
        "44ba0acfb5dceb57be4ad8faf2da32a982507996",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "89": [
        "eabdce17e4937bb0ca6badfeaa934c8ae1a6cd89",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "90": [
        "6231c78c840390b3f4d8444e2cda847641d1ac2f",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "91": [
        "61a879ffb2eb73f55849d6bd79c6216ceb18693e",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "92": [
        "8e92e1ce0bf3d553ce0de80e9be5de98b5790a12",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "93": [
        "4274eff1cba77e2367ae0da21c034a56291e353a",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "94": [
        "77b630496e58bcd42acffd115a36979d85f9a0e4",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "95": [
        "f007861e0c86d7fa706a12d62a463c2931abae3b",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "96": [
        "a06ff127c02a1cd1d0101b8e3f496a43ab937b36",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "97": [
        "760f477f0ab0157801e8c12db4411ee2a6dcf52f",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "98": [
        "aa182a689ecba0dd5624813786ca493a0b49a742",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "99": [
        "7ea34ad8dfe3dce11be555505b0b22afdabc7959",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "test": [
        "79e6584f3574e22e97dfe17ddf1b9856b3f2284f",
        "b056ffe622e9a6cfb76862c5ecb120c73d4fd99e",
    ],
    "val": [
        "c008c5dcaf03d23962756f92e6cd09a902f20f8b",
        "e67663f832358b645232f0d448d84e4d6e8c65cd",
    ],
}


def load_splits(
    get_local_datasets_dir=storage.default_get_local_datasets_dir
) -> FederatedDataset:
    return storage.load_splits(
        dataset_name=DATASET_NAME,
        dataset_split_hashes=DATASET_SPLIT_HASHES,
        get_local_datasets_dir=get_local_datasets_dir,
    )


def load_split(
    split_id: str,
    split_hashes: Tuple[str, str],
    get_local_datasets_dir=storage.default_get_local_datasets_dir,
):
    assert split_id in set(DATASET_SPLIT_HASHES.keys())

    x_i, y_i = storage.load_split(
        dataset_name=DATASET_NAME,
        split_id=split_id,
        split_hashes=split_hashes,
        local_datasets_dir=get_local_datasets_dir(),
    )

    return x_i, y_i
