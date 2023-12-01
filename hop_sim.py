from dataclasses import dataclass
import numpy as np

import hoppy

# A proxy object for a Tensor with some type
@dataclass
class Tensor:
    dtype: np.number

