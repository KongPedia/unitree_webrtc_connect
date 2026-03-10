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

from unitree_webrtc_connect.constants import RTC_TOPIC, WebRTCConnectionMethod


logging.basicConfig(level=logging.FATAL)


def display_data(message):
    imu_state = message["imu_state"]
    quaternion = imu_state["quaternion"]
    gyroscope = imu_state["gyroscope"]
    accelerometer = imu_state["accelerometer"]
    rpy = imu_state["rpy"]
    temperature = imu_state["temperature"]

    mode = message["mode"]
    progress = message["progress"]
    gait_type = message["gait_type"]
    foot_raise_height = message["foot_raise_height"]
    position = message["position"]
    body_height = message["body_height"]
    velocity = message["velocity"]
    yaw_speed = message["yaw_speed"]
    range_obstacle = message["range_obstacle"]
    foot_force = message["foot_force"]
    foot_position_body = message["foot_position_body"]
    foot_speed_body = message["foot_speed_body"]

    sys.stdout.write("\033[H\033[J")

    print("Go2 Robot Status")
    print("===================")
    print(f"Mode: {mode}")
    print(f"Progress: {progress}")
    print(f"Gait Type: {gait_type}")
    print(f"Foot Raise Height: {foot_raise_height} m")
    print(f"Position: {position}")
    print(f"Body Height: {body_height} m")
    print(f"Velocity: {velocity}")
    print(f"Yaw Speed: {yaw_speed}")
    print(f"Range Obstacle: {range_obstacle}")
    print(f"Foot Force: {foot_force}")
    print(f"Foot Position (Body): {foot_position_body}")
    print(f"Foot Speed (Body): {foot_speed_body}")
    print("-------------------")
    print(f"IMU - Quaternion: {quaternion}")
    print(f"IMU - Gyroscope: {gyroscope}")
    print(f"IMU - Accelerometer: {accelerometer}")
    print(f"IMU - RPY: {rpy}")
    print(f"IMU - Temperature: {temperature}°C")
    sys.stdout.flush()


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="10.2.80.92")
        await conn.connect()

        def sportmodestatus_callback(message):
            current_message = message["data"]
            display_data(current_message)

        conn.datachannel.pub_sub.subscribe(RTC_TOPIC["LF_SPORT_MOD_STATE"], sportmodestatus_callback)
        await asyncio.sleep(3600)

    except ValueError as exc:
        logging.error("An error occurred: %s", exc)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
