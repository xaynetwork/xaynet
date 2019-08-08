from typing import Tuple

from absl import flags

from ..types import FederatedDataset
from . import storage

FLAGS = flags.FLAGS

DATASET_NAME = "fashion_mnist_100p_non_IID"
DATASET_SPLIT_HASHES = {
    "00": [
        "6b47619aa899b4fdf45b4f87c21862cdbe539ef0",
        "8a88bf34b4815ef823768b0a9846a18c91260cde",
    ],
    "01": [
        "1547e3f33b254c956cdc55c42c0638e1d8e7c229",
        "ffaee6b8e9dbe47ba7117578e77a7873eaab86c1",
    ],
    "02": [
        "3754af040a6a073a3a21dfbfedd2da18e730551d",
        "c824bb2ccdbad4b27c0010dadaeeab76a647fc4c",
    ],
    "03": [
        "9f295f78e1839bbd3b6d63ef31a2acde46494b06",
        "43086a991ed8d4aa1e248139ac830a5d3e5fccee",
    ],
    "04": [
        "e8c0d59a9d27e92e0f3d0dc9ab242d67e2386bb1",
        "c0ff8fa29386209195a59583be490dd2678cd570",
    ],
    "05": [
        "a5c8a58eb9474850463f56c7b12d0558d4af2ae5",
        "4b0b6527c21723aea951fa984bef99a1e1adc2cb",
    ],
    "06": [
        "199c530a9913430d06362be32a1df31e2913508e",
        "8f23cc68a2e0b188d6306a82006ef2976abbc77b",
    ],
    "07": [
        "1923344ab12d3e529ae78350d086ab432c0f67b8",
        "3297f2aa94fd5f06da8aca801084008a4f7c5bc7",
    ],
    "08": [
        "43cc5fd08e49547725a944c5f2db40bef2dfbad6",
        "2966d997454d06e5f688c3955d351b29b9097ba4",
    ],
    "09": [
        "bbc30a0ac22f177c088cfd4989ff06787cffc1e1",
        "43086a991ed8d4aa1e248139ac830a5d3e5fccee",
    ],
    "10": [
        "969aa763d9c7d6e13db4b23545eecff542eeac27",
        "12c915d828e87fc5e0e0d2f4abd24425dce44827",
    ],
    "11": [
        "65e2076fd1e55a5fbd5bef9be42be5694eaf5113",
        "96b2bc7df94f28dd7d4b5a19fa5f6e559595f603",
    ],
    "12": [
        "bfd5c0d84b5fcf16db3f4920f835530b4a372975",
        "fcd6f69e67e0d94f83d1e1670fec5e1f248e249f",
    ],
    "13": [
        "af5999374874dea1aa595df1bce925077ecfcb89",
        "96b2bc7df94f28dd7d4b5a19fa5f6e559595f603",
    ],
    "14": [
        "79953c5b48f36c67c4426e72138789befb4f1d20",
        "b66105f61e79f9e1528ca5a9e63d15da1b0df9ce",
    ],
    "15": [
        "c219139bec3d6ea54bce54fad3b14018cb8260be",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "16": [
        "753f91791f1e7e5120832449aa73315ba38882d7",
        "5334bfd2e8d33c4e7ff00f3599d555d1dc7af4bb",
    ],
    "17": [
        "65722a05e426f80b5ce54380da54919ca6a95f85",
        "51b39a8dc4528df2ccbbdadf2f38590862eccc4c",
    ],
    "18": [
        "9b8a25f2995e05c25f2995819fa11ae5060ac191",
        "2853df202922bc6bf1f774f11dec29b640af7708",
    ],
    "19": [
        "de0121a02e1db2714609747fe5f38059093dfaa7",
        "839d8b8294f6283ddda5214fdd51ed6e02999cfc",
    ],
    "20": [
        "bed70b5691936a5a6d66a1b7461f9a89af583fa1",
        "0c72795b7db2268f261283912967da2e4acc2708",
    ],
    "21": [
        "796768dab6fc44553eb8600d02c185f2a3ab7ed1",
        "ed56341d40e4e9d3186171457e022ffebfd15966",
    ],
    "22": [
        "bc9f69e1bbbddea0e26bd08fd38d77afb4835e55",
        "a4a8144cd01a277970667b3bc3d73497b497037e",
    ],
    "23": [
        "ed44d5253c640e1a7b93226d4daf0324ffa22d75",
        "6273204bae5cf28c94987206f5a00343223c6612",
    ],
    "24": [
        "b7513f29423f07aa4c458044157f9f54a077e989",
        "96b2bc7df94f28dd7d4b5a19fa5f6e559595f603",
    ],
    "25": [
        "5de6beb45eea0fede2a77b01462751c1202957c5",
        "8927463a9798ab07652874a087e8b4626b96f1b5",
    ],
    "26": [
        "740cd22b090358a8c04f5318de7e8da74d11bf14",
        "be1582472f2865bfcdc82e52c36dc5822b288c4f",
    ],
    "27": [
        "9ff6bc7165966efb5b54a304085f30418310a2c0",
        "1d127877e1685074ffc11ed499db46dac2229b48",
    ],
    "28": [
        "212886af8a3a80d39f533347eebdc3b14585db8b",
        "0c45193c59d843a804d7069a8e8d3ed6dc55ca65",
    ],
    "29": [
        "a927b9ad2d4fc39876234d693045ebd95d24335d",
        "318d4fb2ee28d35989c2c3a96dddd6b1b4813930",
    ],
    "30": [
        "9b83977a2bcb384baf8216cd8b3f1c7f47c90d17",
        "e5a4d3bd1eecca0d09bd857afa93ffb744b40925",
    ],
    "31": [
        "9389e2d882a4f87cd392153b7c00d23aeae07ae0",
        "43086a991ed8d4aa1e248139ac830a5d3e5fccee",
    ],
    "32": [
        "764a42505d28674268b027037720bb2e74cb3ba7",
        "bae0a17f3e8ee602fc747e713f47976857a2af36",
    ],
    "33": [
        "37731f9260756577447eda70b9e6cf12097ad610",
        "e5cf51c11a0e605e0aaf0914bd2b517ede2b7317",
    ],
    "34": [
        "6b9114206f52cdfcf52e036f7b75a7a1aa9294b4",
        "8927463a9798ab07652874a087e8b4626b96f1b5",
    ],
    "35": [
        "0f0e5bbe78c8884c1ad71488ca13313135bff36f",
        "bb59a4c9414a92023797ed38606da28285b72e1e",
    ],
    "36": [
        "f9e33a15e4bbc676de8b254d3433a35cc2d5fc21",
        "a15440c87de62c02ed88835e080824558749337d",
    ],
    "37": [
        "ce414786cb0657c256e351d19c39490e8dd42488",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "38": [
        "51844f34a97d95a7f33eaae6dd584fb0c0e4269d",
        "016971b7e77cee99b4a52a4252e848e8bc58a232",
    ],
    "39": [
        "561ebece7ed88060575f8a4f805e1a6080a14f0e",
        "7a00c49d0ffc34d6cdb2f1237db52b21f17247b2",
    ],
    "40": [
        "78779ffe17039c3046d195c2bdd4f3fc3786ba04",
        "8927463a9798ab07652874a087e8b4626b96f1b5",
    ],
    "41": [
        "2d5cc7fbea9e9d24daa68c70e992fe986f86e91e",
        "e5a4d3bd1eecca0d09bd857afa93ffb744b40925",
    ],
    "42": [
        "f25d8b6368a6ba7d67a3ab18d84ffeaa6656954c",
        "e5a4d3bd1eecca0d09bd857afa93ffb744b40925",
    ],
    "43": [
        "ee6d856d1c6548a3ce66a8033150c36bbeea2e2d",
        "928795e503daa559aa11e9d046767a8e5522d94c",
    ],
    "44": [
        "5364504a326130522e099c206ce03e3f23e1dcf3",
        "bb59a4c9414a92023797ed38606da28285b72e1e",
    ],
    "45": [
        "2be4c7b30c8e614c63a80a88a0c410a043ac0814",
        "b55f0533a9bffce4b61e3a8190ef95e33fc7fee6",
    ],
    "46": [
        "68cd31d4ef93496f69159a475be5a1c0da5608d3",
        "c9849f81a7e41091012fd887791087fd6fbc182a",
    ],
    "47": [
        "aa601e96d527372efb334e20d060aa5fce9493fd",
        "99822171b0d75274dffa48388786fd8d8bee3b5d",
    ],
    "48": [
        "6bff50f1b6e9e240c68a09df2eaeb2d013e38411",
        "528baecfb238d6e04ac17709abc732a56e14e5af",
    ],
    "49": [
        "1635c3895cce5b6a86c9f654b3a0a3cb5dd445f7",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "50": [
        "65984f146ee6cd56b365633c1814b0459efb6463",
        "8f3e5bb0cb531e3e1c4a14d30b44606e80765186",
    ],
    "51": [
        "07fcc839427656b08a06e68de8f7166142df9229",
        "5334bfd2e8d33c4e7ff00f3599d555d1dc7af4bb",
    ],
    "52": [
        "1a24f9dc3105fe7706d939f2742ead90eb6e6e5d",
        "9213d1d5ddb7c405e58c4a7669b8a43c7ab21a25",
    ],
    "53": [
        "fddd785be6a7f1e264aae4ea38ccb05c94500237",
        "839d8b8294f6283ddda5214fdd51ed6e02999cfc",
    ],
    "54": [
        "625ccc8f630d5450f73c4e11d83ea6c6b81ccee6",
        "5e58323803dd23683d3732f5ce0379dbe3a23ae9",
    ],
    "55": [
        "760369247d8591d89e0cc39554c3115bdd9048f8",
        "e5a4d3bd1eecca0d09bd857afa93ffb744b40925",
    ],
    "56": [
        "64a1fd3e586996d19b05576fc547964a34332e71",
        "95c7c4af113667f0cfe3ba689f75d92275eda152",
    ],
    "57": [
        "60fdee56b372962deb0a9e1507d38258f8ec521c",
        "b4bf7d89b08cb50787fa0fce9c6d0c306822a606",
    ],
    "58": [
        "ef732bd2f50757b8e24a4b4c242f1462ea9f968e",
        "f9b7a73e153827ee60121a768b17e00b42109729",
    ],
    "59": [
        "58e325171af385e3a6e65c5f018fb1a4def8ee47",
        "b4bf7d89b08cb50787fa0fce9c6d0c306822a606",
    ],
    "60": [
        "72023d65d32af795879402f6332849469ac20e2d",
        "51b39a8dc4528df2ccbbdadf2f38590862eccc4c",
    ],
    "61": [
        "60a73b0bcd223b58063f61af6d7f9361cee09e40",
        "928795e503daa559aa11e9d046767a8e5522d94c",
    ],
    "62": [
        "1c3c0925b3e15e23f51bf6534fdfc57d37924825",
        "6516c662e232cf34939344524849b207820f0f67",
    ],
    "63": [
        "9a4f959b0a65429de8845d89a90fd8514b257b66",
        "c9849f81a7e41091012fd887791087fd6fbc182a",
    ],
    "64": [
        "74c1d7fc87190acf49fefec0a4bdf94ebf615234",
        "be1582472f2865bfcdc82e52c36dc5822b288c4f",
    ],
    "65": [
        "0d05c63ff889e2d086df59e38ef39dfc4e0496fa",
        "efe2785947cc324c4de23bb6797620e799faa92e",
    ],
    "66": [
        "6d1941d2724e312404d621d8e6017c912904dbb0",
        "fcefa536c15bc66497b225e998d7f5b553340cce",
    ],
    "67": [
        "e496abe226f7d8e3e018e60c5a690d5f14d3bfa1",
        "c8ff6452bfaa42161a1fb81500ccbf9d1982af55",
    ],
    "68": [
        "3c5cff08cd2ba0cac76d17e97b2786d945f43fc1",
        "8322f2289abfc43a90d2f13e837ca8cd437b6ff0",
    ],
    "69": [
        "88fb9388275480202acde8d001ee21c2ed4c45e7",
        "efe2785947cc324c4de23bb6797620e799faa92e",
    ],
    "70": [
        "b9f5e5ff28b55fece6fb972cb9ffa9f12f25d45d",
        "9358526e0b402ae1b1653f74a0a9c20a8d8c4c8e",
    ],
    "71": [
        "c75f116ff6748b1e36ef52846036b5433b1dd34d",
        "c5ecbc7d1aeb28befcbf9b9af4bba2cc27a6b8de",
    ],
    "72": [
        "522b1fc8fcda07ef3a76093c38b255b771f4085f",
        "2eb513b18ac253ccd6c0372f6a369401c6680713",
    ],
    "73": [
        "75d9d9b964f872a0cdbdec40b45c8b02d8e74130",
        "5334bfd2e8d33c4e7ff00f3599d555d1dc7af4bb",
    ],
    "74": [
        "9b050a78084d6dd10fe31f6268e09ad0ef7d142a",
        "ab81504f9d7350f7c1b0e1b49eadafc910d1005b",
    ],
    "75": [
        "5048a886e48ca8dc6ab08109de79925792b0b312",
        "1b097bf395a9f5bb6c83bae85a746dccd930a9cc",
    ],
    "76": [
        "8c16cc20486b25b87b6954b9947a1f3eb9808f34",
        "677ce9c834f18e1280cf8a02dbf7cfbc8e75f332",
    ],
    "77": [
        "ff8177761ae5fbc20e966fe37579b4a98cb65655",
        "a1a61cdff7503b4eff80d98ca014283344900009",
    ],
    "78": [
        "eaef2e242292e6550ed78398e2a6838a49978d8e",
        "0c72795b7db2268f261283912967da2e4acc2708",
    ],
    "79": [
        "8bdd2d52ce514266233f5d15db0114700dc11ef5",
        "fba92bde2718e726d5c3f6413800caa664108826",
    ],
    "80": [
        "fedb19bedf56d2cd51c95eb05970c78cab94d0e4",
        "9358526e0b402ae1b1653f74a0a9c20a8d8c4c8e",
    ],
    "81": [
        "691539eccd990e8c27c96cc0122d94cf2e580b6f",
        "e5cf51c11a0e605e0aaf0914bd2b517ede2b7317",
    ],
    "82": [
        "36cb4885e99026e6ad21788b9f70742556d0210c",
        "955cba181bbfae241f694f66adfdcc38217319da",
    ],
    "83": [
        "909ace218dd5b70a698833a7d64682e77c7c1f2f",
        "ed3267ecfc1f7a3cfb3fe149d50ca8abfe2bdbd4",
    ],
    "84": [
        "1a7aefef5683d4a83529d7af1384ad024bf72487",
        "e70ea47cbe6a743c416e5fabf9dbc1d9c50f0a45",
    ],
    "85": [
        "5bd149b905ad27e1596e50a775f971297346f10e",
        "8686fdb97d764eca74343ac0dd885000aa24312f",
    ],
    "86": [
        "59c7a620f3347aef801ac8c63f612c95cae32fb0",
        "b66105f61e79f9e1528ca5a9e63d15da1b0df9ce",
    ],
    "87": [
        "f3c87b4a198264f51fe1139bc415fff1bc288f6c",
        "318d4fb2ee28d35989c2c3a96dddd6b1b4813930",
    ],
    "88": [
        "0906d4aa030026e35bd8ba4c5b5cd0476e4dd866",
        "fcd6f69e67e0d94f83d1e1670fec5e1f248e249f",
    ],
    "89": [
        "b24cb0a51ef82f38aabd543b62edd9fdb5d3054f",
        "0c72795b7db2268f261283912967da2e4acc2708",
    ],
    "90": [
        "05db7ae26b5816d9680bb28a15551a0671a94270",
        "ec8b5f16f487e8380d9682d7378aa930bf31e3cd",
    ],
    "91": [
        "6b8902287a63844cd21b53948fa903bbfb10d22b",
        "2853df202922bc6bf1f774f11dec29b640af7708",
    ],
    "92": [
        "a334a61653557ea2088b8e27dda8045f1ea38a1b",
        "2eb513b18ac253ccd6c0372f6a369401c6680713",
    ],
    "93": [
        "d4c01de5c53a5adf341bdf48a00ac667fa4e52ae",
        "d92b210993a5f1e0290c3d9d907877227c05f295",
    ],
    "94": [
        "c550d220cc15c3dba613550e6d939b7f8e57229a",
        "1d127877e1685074ffc11ed499db46dac2229b48",
    ],
    "95": [
        "11cdf2c2f55d68392ea0e4d61231ffc2ccaa4ac8",
        "8f98e60081ac99fb246d97b5fcb27ac35016b66f",
    ],
    "96": [
        "323002ebb4f130a80b9d67862387251640a02a0c",
        "ec8b5f16f487e8380d9682d7378aa930bf31e3cd",
    ],
    "97": [
        "51113902a107e73227e1ba42753139d9bdde8161",
        "ec8b5f16f487e8380d9682d7378aa930bf31e3cd",
    ],
    "98": [
        "503a14b2edb9abe4b7d685ba0ec5937acdffd4fb",
        "27b294266f3f5952d7c16dc75ad242c13e79eb02",
    ],
    "99": [
        "89c7bc468f83716bcb6c1ebc40364b152ba614d2",
        "ed3267ecfc1f7a3cfb3fe149d50ca8abfe2bdbd4",
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
