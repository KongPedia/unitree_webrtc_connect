import asyncio
import logging
import json
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

from unitree_webrtc_connect.constants import WebRTCConnectionMethod, RTC_TOPIC, SPORT_CMD

# Enable logging for debugging
logging.basicConfig(level=logging.FATAL)


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="10.2.80.92")
        await conn.connect()

        print("Checking current motion mode...")

        response = await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["MOTION_SWITCHER"],
            {"api_id": 1001},
        )

        if response["data"]["header"]["status"]["code"] == 0:
            data = json.loads(response["data"]["data"])
            current_motion_switcher_mode = data["name"]
            print(f"Current motion mode: {current_motion_switcher_mode}")

        # Switch to "normal" mode if not already
        if current_motion_switcher_mode != "normal":
            print(f"Switching motion mode from {current_motion_switcher_mode} to 'normal'...")
            await conn.datachannel.pub_sub.publish_request_new(
                RTC_TOPIC["MOTION_SWITCHER"],
                {
                    "api_id": 1002,
                    "parameter": {"name": "normal"},
                },
            )
            await asyncio.sleep(5)  # Wait while it stands up

        print("Performing 'StandUp' movement...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {"api_id": SPORT_CMD["StandUp"]},
        )

        print("Performing 'recovery' movement...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {"api_id": SPORT_CMD["RecoveryStand"]},
        )

        await asyncio.sleep(1)

        print("Moving forward...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {
                "api_id": SPORT_CMD["Move"],
                "parameter": {"x": 0.5, "y": 0, "z": 0},
            },
        )

        await asyncio.sleep(3)

        print("Moving backward...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {
                "api_id": SPORT_CMD["Move"],
                "parameter": {"x": -0.5, "y": 0, "z": 0},
            },
        )

        await asyncio.sleep(3)

        print("Performing 'Hello' movement...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {"api_id": SPORT_CMD["Hello"]},
        )

        print("Switching to FrontFlip Mode...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {
                "api_id": SPORT_CMD["FrontFlip"],
                "parameter": {"data": True},
            },
        )

        print("Switching to Handstand Mode...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"],
            {
                "api_id": SPORT_CMD["StandDown"],
                "parameter": {"data": True},
            },
        )

        # Keep the program running for a while
        await asyncio.sleep(3600)

    except ValueError as e:
        logging.error(f"An error occurred: {e}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
