import os

resource_path = os.path.join(
    os.path.dirname(os.path.realpath(__file__)), "..", "..", "..", "data",
)
if not os.path.exists(resource_path):
    from data_files import find_data_files
    resource_path = find_data_files("omim-data")
    assert resource_path is not None


from feature_types import init as _init

_init(resource_path)

from feature_types import INDEX_TO_NAME_TYPE_MAPPING
from feature_types import NAME_TO_INDEX_TYPE_MAPPING
from feature_types import readable_type
from feature_types import type_index
from mwm_interface import GeomType
from mwm_interface import MapType
from mwm_interface import MetadataField
from mwm_interface import Point
from mwm_interface import Rect
from mwm_interface import RegionDataField
from mwm_interface import Triangle
from mwm_python import get_region_info
from utils import EnumAsStrEncoder

try:
    from mwm_pygen import MwmPygen as Mwm
    from mwm_pygen import FeaturePygen as Feature
    from mwm_pygen import init as _init

    _init(resource_path)
except ImportError:
    from mwm_python import MwmPython as Mwm
    from mwm_python import FeaturePython as Feature
