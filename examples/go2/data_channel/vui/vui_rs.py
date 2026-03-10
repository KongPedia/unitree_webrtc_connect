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

from unitree_webrtc_connect.constants import RTC_TOPIC, VUI_COLOR, WebRTCConnectionMethod


logging.basicConfig(level=logging.FATAL)


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.8.181")
        await conn.connect()

        print("\nFetching the current brightness level...")
        response = await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["VUI"],
            {"api_id": 1006},
        )

        if response["data"]["header"]["status"]["code"] == 0:
            data = json.loads(response["data"]["data"])
            current_brightness = data["brightness"]
            print(f"Current brightness level: {current_brightness}\n")

        print("Increasing brightness from 0 to 10...")
        for brightness_level in range(0, 11):
            await conn.datachannel.pub_sub.publish_request_new(
                RTC_TOPIC["VUI"],
                {
                    "api_id": 1005,
                    "parameter": {"brightness": brightness_level},
                },
            )
            print(f"Brightness level: {brightness_level}/10")
            await asyncio.sleep(0.5)

        print("\nDecreasing brightness from 10 to 0...")
        for brightness_level in range(10, -1, -1):
            await conn.datachannel.pub_sub.publish_request_new(
                RTC_TOPIC["VUI"],
                {
                    "api_id": 1005,
                    "parameter": {"brightness": brightness_level},
                },
            )
            print(f"Brightness level: {brightness_level}/10")
            await asyncio.sleep(0.5)

        print("\nChanging LED color to purple for 5 seconds...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["VUI"],
            {
                "api_id": 1007,
                "parameter": {
                    "color": VUI_COLOR.PURPLE,
                    "time": 5,
                },
            },
        )
        await asyncio.sleep(6)

        print("\nChanging LED color to cyan with flash (cycle: 1000ms)...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["VUI"],
            {
                "api_id": 1007,
                "parameter": {
                    "color": VUI_COLOR.CYAN,
                    "time": 5,
                    "flash_cycle": 1000,
                },
            },
        )
        await asyncio.sleep(5)

        print("\nFetching the current volume level...")
        response = await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["VUI"],
            {"api_id": 1004},
        )

        if response["data"]["header"]["status"]["code"] == 0:
            data = json.loads(response["data"]["data"])
            current_volume = data["volume"]
            print(f"Current volume level: {current_volume}/10\n")

        print("Setting volume to 50% (5/10)...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["VUI"],
            {
                "api_id": 1003,
                "parameter": {"volume": 5},
            },
        )

        await asyncio.sleep(3600)

    except ValueError as exc:
        logging.error("An error occurred: %s", exc)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
