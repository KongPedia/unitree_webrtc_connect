import asyncio
import logging
import sys
import warnings

try:
    from unitree_webrtc_connect import UnitreeWebRTCConnection
except ImportError:
    warnings.warn(
        "webrtc_driver_rs import failed. Falling back to python webrtc_driver.",
        RuntimeWarning,
    )
    from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection

from unitree_webrtc_connect.constants import WebRTCConnectionMethod


logging.basicConfig(level=logging.FATAL)


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.8.181")

        await conn.connect()
        await conn.datachannel.disableTrafficSaving(True)
        conn.datachannel.set_decoder(decoder_type="libvoxel")
        conn.datachannel.pub_sub.publish_without_callback("rt/utlidar/switch", "on")

        def lidar_callback(message):
            print(message["data"])

        conn.datachannel.pub_sub.subscribe("rt/utlidar/voxel_map_compressed", lidar_callback)
        await asyncio.sleep(3600)

    except ValueError as exc:
        logging.error("An error occurred: %s", exc)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
