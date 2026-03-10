import asyncio
import json
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

from unitree_webrtc_connect.constants import RTC_TOPIC, WebRTCConnectionMethod


logging.basicConfig(level=logging.FATAL)


def display_data(message):
    message = json.loads(message)

    body_height = message["bodyHeight"]
    brightness = message["brightness"]
    foot_raise_height = message["footRaiseHeight"]
    obstacles_avoid_switch = message["obstaclesAvoidSwitch"]
    speed_level = message["speedLevel"]
    uwb_switch = message["uwbSwitch"]
    volume = message["volume"]

    sys.stdout.write("\033[H\033[J")

    print("Go2 Multiple Robot Status")
    print("===================")
    print(f"Body Height:           {body_height:.2f} meters")
    print(f"Brightness:            {brightness}")
    print(f"Foot Raise Height:     {foot_raise_height:.2f} meters")
    print(f"Obstacles Avoid Switch: {'Enabled' if obstacles_avoid_switch else 'Disabled'}")
    print(f"Speed Level:           {speed_level}")
    print(f"UWB Switch:            {'On' if uwb_switch else 'Off'}")
    print(f"Volume:                {volume}/10")
    print("===================")

    sys.stdout.flush()


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.8.181")
        await conn.connect()

        def multiplestate_callback(message):
            current_message = message["data"]
            display_data(current_message)

        conn.datachannel.pub_sub.subscribe(RTC_TOPIC["MULTIPLE_STATE"], multiplestate_callback)
        await asyncio.sleep(3600)

    except ValueError as exc:
        logging.error("An error occurred: %s", exc)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
