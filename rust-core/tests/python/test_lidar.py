import unittest
from unitree_webrtc_core_rs import UnifiedLidarDecoder
import numpy as np

class TestLidarDecoder(unittest.TestCase):
    def test_init(self):
        decoder = UnifiedLidarDecoder()
        self.assertIsNotNone(decoder)

    def test_native_decoder_invalid_data(self):
        decoder = UnifiedLidarDecoder()
        decoder.set_decoder("native")
        # Native decoder requires valid lz4 payload, so we test if it catches bad data
        with self.assertRaises(ValueError):
            decoder.decode(b"invalid_data_not_lz4", [0.0, 0.0, 0.0], 0.05)

    def test_set_notfound_decoder(self):
        decoder = UnifiedLidarDecoder()
        with self.assertRaises(ValueError):
            decoder.set_decoder("notfound")

if __name__ == '__main__':
    unittest.main()
