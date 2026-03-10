import asyncio
import logging
import sys
import warnings

try:
    from unitree_webrtc_connect.webrtc_driver_rs import UnitreeWebRTCConnection
except ImportError:
    warnings.warn(
        "webrtc_driver_rs import failed. Falling back to python webrtc_driver.",
        RuntimeWarning,
    )
    from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection

from unitree_webrtc_connect.constants import RTC_TOPIC, WebRTCConnectionMethod


logging.basicConfig(level=logging.FATAL)


def display_data(message):
    imu_state = message["imu_state"]["rpy"]
    motor_state = message["motor_state"]
    bms_state = message["bms_state"]
    foot_force = message["foot_force"]
    temperature_ntc1 = message["temperature_ntc1"]
    power_v = message["power_v"]

    sys.stdout.write("\033[H\033[J")

    print("Go2 Robot Status (LowState)")
    print("===========================")
    print(f"IMU - RPY: Roll: {imu_state[0]}, Pitch: {imu_state[1]}, Yaw: {imu_state[2]}")

    print("\nMotor States (q, Temperature, Lost):")
    print("------------------------------------------------------------")
    for i, motor in enumerate(motor_state):
        print(f"Motor {i + 1:2}: q={motor['q']:.4f}, Temp={motor['temperature']}°C, Lost={motor['lost']}")

    print("\nBattery Management System (BMS) State:")
    print(f"  Version: {bms_state['version_high']}.{bms_state['version_low']}")
    print(f"  SOC (State of Charge): {bms_state['soc']}%")
    print(f"  Current: {bms_state['current']} mA")
    print(f"  Cycle Count: {bms_state['cycle']}")
    print(f"  BQ NTC: {bms_state['bq_ntc']}°C")
    print(f"  MCU NTC: {bms_state['mcu_ntc']}°C")

    print(f"\nFoot Force: {foot_force}")
    print(f"Temperature NTC1: {temperature_ntc1}°C")
    print(f"Power Voltage: {power_v}V")
    sys.stdout.flush()


async def main():
    try:
        conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.8.181")
        await conn.connect()

        def lowstate_callback(message):
            current_message = message["data"]
            display_data(current_message)

        conn.datachannel.pub_sub.subscribe(RTC_TOPIC["LOW_STATE"], lowstate_callback)
        await asyncio.sleep(3600)

    except ValueError as exc:
        logging.error("An error occurred: %s", exc)


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nProgram interrupted by user")
        sys.exit(0)
