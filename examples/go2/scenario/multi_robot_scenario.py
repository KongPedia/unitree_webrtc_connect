from __future__ import annotations

import argparse
import asyncio
import contextlib
import json
import logging
from pathlib import Path
from typing import Any, Literal

from pydantic import BaseModel, Field, ValidationError, field_validator, model_validator

import random
import sys

try:
    import yaml
except ImportError:
    yaml = None

ActionName = Literal["liedown", "red", "standup", "stop", "damp", "common"]
ACTION_NAMES = {"liedown", "red", "standup", "stop", "damp", "common"}

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")


def get_webrtc_constants() -> tuple[dict[str, str], dict[str, int], Any, Any]:
    from unitree_webrtc_connect.constants import RTC_TOPIC, SPORT_CMD, VUI_COLOR, WebRTCConnectionMethod

    return RTC_TOPIC, SPORT_CMD, VUI_COLOR, WebRTCConnectionMethod


def get_connection_class() -> Any:
    from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection

    return UnitreeWebRTCConnection


class RobotConfig(BaseModel):
    ip: str = Field(min_length=1)

    @field_validator("ip")
    @classmethod
    def validate_ip(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Robot IP must not be empty")
        return normalized


class SwayConfig(BaseModel):
    left_seconds: float = Field(default=2.0, gt=0)
    right_seconds: float = Field(default=2.0, gt=0)
    cycles: int = Field(default=3, ge=1)
    lateral_value: float = Field(default=0.6, ge=0.0, le=1.0)
    publish_interval_seconds: float = Field(default=0.1, gt=0.0)


class StepSettings(BaseModel):
    red_duration_seconds: int = Field(default=3600, ge=1)
    liedown_settle_seconds: float = Field(default=3.0, ge=0.0)
    standup_settle_seconds: float = Field(default=2.0, ge=0.0)
    recovery_delay_seconds: float = Field(default=1.0, ge=0.0)
    sway: SwayConfig = Field(default_factory=SwayConfig)


class ScenarioConfig(BaseModel):
    robots: dict[str, RobotConfig]
    groups: dict[str, list[str]] = Field(default_factory=dict)
    startup_target: str = Field(default="all", min_length=1)
    startup_sequence: list[ActionName] = Field(default_factory=lambda: ["liedown"])
    steps: StepSettings = Field(default_factory=StepSettings)

    @field_validator("robots")
    @classmethod
    def validate_robots(cls, value: dict[str, RobotConfig]) -> dict[str, RobotConfig]:
        if not value:
            raise ValueError("At least one robot must be configured")
        return value

    @model_validator(mode="after")
    def validate_groups(self) -> "ScenarioConfig":
        normalized_groups: dict[str, list[str]] = {}
        for group_name, members in self.groups.items():
            cleaned_group_name = group_name.strip()
            if not cleaned_group_name:
                raise ValueError("Group names must not be empty")
            normalized_groups[cleaned_group_name] = []
            for member in members:
                if member not in self.robots:
                    raise ValueError(f"Unknown robot '{member}' in group '{cleaned_group_name}'")
                if member not in normalized_groups[cleaned_group_name]:
                    normalized_groups[cleaned_group_name].append(member)
        normalized_groups.setdefault("all", list(self.robots.keys()))
        if self.startup_target not in normalized_groups and self.startup_target not in self.robots:
            raise ValueError(f"Unknown startup target '{self.startup_target}'")
        self.groups = normalized_groups
        return self


class ScenarioCommand(BaseModel):
    step: ActionName
    source: Literal["startup", "interactive"] = "interactive"


class RobotSession:
    def __init__(self, robot_id: str, robot_config: RobotConfig, steps: StepSettings, backend: str | None) -> None:
        self.robot_id = robot_id
        self.robot_config = robot_config
        self.steps = steps
        self.backend = backend
        self.connection: Any | None = None
        self.queue: asyncio.Queue[ScenarioCommand | None] = asyncio.Queue()
        self.worker_task: asyncio.Task[None] | None = None
        self.motion_task: asyncio.Task[None] | None = None
        self.connected = False

    async def connect(self) -> None:
        _, _, _, connection_method = get_webrtc_constants()
        connection_class = get_connection_class()
        kwargs = {"ip": self.robot_config.ip}
        if self.backend:
            kwargs["backend"] = self.backend
        self.connection = connection_class(connection_method.LocalSTA, **kwargs)
        logging.info("[%s] connecting to %s", self.robot_id, self.robot_config.ip)
        await self.connection.connect()
        self.connected = True
        await self.ensure_normal_mode()
        self.worker_task = asyncio.create_task(self.worker(), name=f"scenario-worker-{self.robot_id}")
        logging.info("[%s] connected", self.robot_id)

    async def close(self) -> None:
        await self.queue.put(None)
        if self.worker_task is not None:
            await self.worker_task
        await self.interrupt_motion()
        if self.connection is not None and self.connected:
            await self.connection.disconnect()
        self.connected = False
        logging.info("[%s] disconnected", self.robot_id)

    async def enqueue(self, command: ScenarioCommand) -> None:
        await self.queue.put(command)

    async def wait_until_idle(self, include_motion: bool = False) -> None:
        await self.queue.join()
        if include_motion and self.motion_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self.motion_task

    async def worker(self) -> None:
        rtc_topic, sport_cmd, vui_color, _ = get_webrtc_constants()
        while True:
            command = await self.queue.get()
            try:
                if command is None:
                    return
                if command.step != "red":
                    await self.interrupt_motion()
                if command.step == "liedown":
                    await self.request(
                        rtc_topic["VUI"],
                        {
                            "api_id": 1007,
                            "parameter": {
                                "color": vui_color.GREEN,
                                "time": 3600,
                            },
                        },
                    )
                    await self.request(rtc_topic["SPORT_MOD"], {"api_id": sport_cmd["StandDown"]})
                    await asyncio.sleep(self.steps.liedown_settle_seconds)
                elif command.step == "red":
                    # Set color to RED
                    await self.request(
                        rtc_topic["VUI"],
                        {
                            "api_id": 1007,
                            "parameter": {
                                "color": vui_color.RED,
                                "time": self.steps.red_duration_seconds,
                            },
                        },
                    )
                elif command.step == "common":
                    # Set color to GREEN
                    await self.request(
                        rtc_topic["VUI"],
                        {
                            "api_id": 1007,
                            "parameter": {
                                "color": vui_color.GREEN,
                                "time": 3600,
                            },
                        },
                    )
                elif command.step == "standup":
                    # Random delay for standup as requested (0 to 1.5s)
                    await asyncio.sleep(random.uniform(0, 1.5))
                    await self.request(rtc_topic["SPORT_MOD"], {"api_id": sport_cmd["StandUp"]})
                    await asyncio.sleep(self.steps.standup_settle_seconds)
                    await self.request(rtc_topic["SPORT_MOD"], {"api_id": sport_cmd["RecoveryStand"]})
                    await asyncio.sleep(self.steps.recovery_delay_seconds)
                elif command.step == "sway":
                    self.motion_task = asyncio.create_task(self.run_sway(), name=f"scenario-sway-{self.robot_id}")
                elif command.step == "stop":
                    self.send_wireless_controller()
                    await self.request(rtc_topic["SPORT_MOD"], {"api_id": sport_cmd["StopMove"]})
                elif command.step == "damp":
                    self.send_wireless_controller()
                    # Random movement for ~1s before damp
                    self.motion_task = asyncio.create_task(self.run_random_move(), name=f"scenario-random-{self.robot_id}")
                    await self.motion_task
                    await self.request(rtc_topic["SPORT_MOD"], {"api_id": sport_cmd["Damp"]})
                    # Set color to GREEN after damp
                    await self.request(
                        rtc_topic["VUI"],
                        {
                            "api_id": 1007,
                            "parameter": {
                                "color": vui_color.GREEN,
                                "time": 3600,
                            },
                        },
                    )
                # logging.info("[%s] executed step=%s source=%s", self.robot_id, command.step, command.source)
            except Exception:
                logging.exception("[%s] failed to execute scenario step", self.robot_id)
            finally:
                self.queue.task_done()

    async def ensure_normal_mode(self) -> None:
        rtc_topic, _, _, _ = get_webrtc_constants()
        if self.connection is None:
            raise RuntimeError(f"[{self.robot_id}] connection is not initialized")
        response = await self.connection.datachannel.pub_sub.publish_request_new(
            rtc_topic["MOTION_SWITCHER"],
            {"api_id": 1001},
            timeout=5.0,
        )
        current_mode = "unknown"
        with contextlib.suppress(KeyError, TypeError, json.JSONDecodeError):
            if response["data"]["header"]["status"]["code"] == 0:
                current_mode = json.loads(response["data"]["data"]).get("name", "unknown")
        if current_mode == "normal":
            return
        # logging.info("[%s] switching motion mode from %s to normal", self.robot_id, current_mode)
        await self.connection.datachannel.pub_sub.publish_request_new(
            rtc_topic["MOTION_SWITCHER"],
            {"api_id": 1002, "parameter": {"name": "normal"}},
            timeout=5.0,
        )
        await asyncio.sleep(5)

    async def request(self, topic: str, payload: dict) -> dict:
        if self.connection is None:
            raise RuntimeError(f"[{self.robot_id}] connection is not initialized")
        return await self.connection.datachannel.pub_sub.publish_request_new(topic, payload, timeout=5.0)

    def send_wireless_controller(self, lx: float = 0.0, ly: float = 0.0, rx: float = 0.0, ry: float = 0.0) -> None:
        rtc_topic, _, _, _ = get_webrtc_constants()
        if self.connection is None:
            raise RuntimeError(f"[{self.robot_id}] connection is not initialized")
        self.connection.datachannel.pub_sub.publish_without_callback(
            rtc_topic["WIRELESS_CONTROLLER"],
            {"lx": lx, "ly": ly, "rx": rx, "ry": ry, "keys": 0},
        )

    async def interrupt_motion(self) -> None:
        if self.motion_task is not None and not self.motion_task.done():
            self.motion_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self.motion_task
        self.motion_task = None
        if self.connected:
            self.send_wireless_controller()

    async def run_random_move(self) -> None:
        # Move in a random direction every 0.2s for 1s
        deadline = asyncio.get_running_loop().time() + 1.0
        while True:
            lx = random.uniform(-0.5, 0.5)
            ly = random.uniform(-0.5, 0.5)
            self.send_wireless_controller(lx=lx, ly=ly)
            remaining = deadline - asyncio.get_running_loop().time()
            if remaining <= 0:
                break
            await asyncio.sleep(min(0.2, remaining))

    async def publish_sway_direction(self, lx: float, ly: float, duration_seconds: float) -> None:
        deadline = asyncio.get_running_loop().time() + duration_seconds
        while True:
            self.send_wireless_controller(lx=lx, ly=ly)
            remaining = deadline - asyncio.get_running_loop().time()
            if remaining <= 0:
                break
            await asyncio.sleep(min(self.steps.sway.publish_interval_seconds, remaining))


class ScenarioManager:
    def __init__(self, config: ScenarioConfig, backend: str | None) -> None:
        self.config = config
        self.sessions = {
            robot_id: RobotSession(robot_id, robot_config, config.steps, backend)
            for robot_id, robot_config in config.robots.items()
        }

    def resolve_target(self, target: str) -> list[str]:
        if target in self.config.groups:
            return self.config.groups[target]
        if target in self.sessions:
            return [target]
        raise ValueError(f"Unknown target '{target}'")

    async def connect_all(self) -> None:
        robot_ids = list(self.sessions.keys())
        results = await asyncio.gather(*(self.sessions[robot_id].connect() for robot_id in robot_ids), return_exceptions=True)
        connected_robot_ids = []
        for robot_id, result in zip(robot_ids, results, strict=True):
            if isinstance(result, Exception):
                logging.error("[%s] connection failed: %s", robot_id, result)
            else:
                connected_robot_ids.append(robot_id)
        if not connected_robot_ids:
            raise RuntimeError("No robots connected successfully")
        logging.info("Connected robots: %s", ", ".join(connected_robot_ids))

    async def close(self) -> None:
        connected_sessions = [session for session in self.sessions.values() if session.connected]
        if connected_sessions:
            await asyncio.gather(*(session.close() for session in connected_sessions), return_exceptions=True)

    async def dispatch(self, target: str, step: ActionName, source: Literal["startup", "interactive"], wait: bool = False) -> None:
        resolved_robot_ids = [robot_id for robot_id in self.resolve_target(target) if self.sessions[robot_id].connected]
        if not resolved_robot_ids:
            raise RuntimeError(f"Target '{target}' has no connected robots")
        command = ScenarioCommand(step=step, source=source)
        await asyncio.gather(*(self.sessions[robot_id].enqueue(command) for robot_id in resolved_robot_ids))
        # logging.info("Dispatched step=%s target=%s robots=%s", step, target, ", ".join(resolved_robot_ids))
        if wait:
            await asyncio.gather(*(self.sessions[robot_id].wait_until_idle(include_motion=False) for robot_id in resolved_robot_ids))

    async def run_startup_sequence(self) -> None:
        for step in self.config.startup_sequence:
            await self.dispatch(self.config.startup_target, step, source="startup", wait=True)

    def status_report(self) -> str:
        lines = []
        for robot_id, session in self.sessions.items():
            lines.append(
                f"{robot_id}: connected={session.connected} ip={session.robot_config.ip} queued={session.queue.qsize()} moving={session.motion_task is not None and not session.motion_task.done()}"
            )
        return "\n".join(lines)


def load_config(config_path: Path) -> ScenarioConfig:
    raw_text = config_path.read_text(encoding="utf-8")
    if config_path.suffix.lower() == ".json":
        payload = json.loads(raw_text)
    elif config_path.suffix.lower() in {".yml", ".yaml"}:
        if yaml is None:
            raise RuntimeError("PyYAML is required for YAML configs. Install it or use JSON config.")
        payload = yaml.safe_load(raw_text)
    else:
        raise ValueError("Config file must be .json, .yml, or .yaml")
    return ScenarioConfig.model_validate(payload)


def parse_cli_command(raw_command: str) -> tuple[str, ActionName]:
    parts = raw_command.strip().split()
    if len(parts) == 1 and parts[0].lower() in ACTION_NAMES:
        return "all", parts[0].lower()  # type: ignore[return-value]
    if len(parts) != 2:
        raise ValueError("Command must be '<target> <step>' or '<step>'")
    target, step = parts[0], parts[1].lower()
    if step not in ACTION_NAMES:
        raise ValueError(f"Unknown step '{step}'")
    return target, step  # type: ignore[return-value]


async def interactive_loop(manager: ScenarioManager) -> None:
    print("Commands: <target> <step>, <step>, status, teleop, help, exit")
    print("Steps: liedown, red, standup, stop, damp, common (sway is hidden)")
    while True:
        try:
            raw_command = await asyncio.to_thread(input, "scenario> ")
        except EOFError:
            return
        command = raw_command.strip()
        if not command:
            continue
        if command.lower() in {"exit", "quit"}:
            return
        if command.lower() == "status":
            print(manager.status_report())
            continue
        if command.lower() == "teleop":
            tel = TeleopManager(manager)
            await tel.run()
            continue
        if command.lower() == "help":
            print("Commands: <target> <step>, <step>, status, teleop, help, exit")
            print("Examples: all red | teamA standup | all stop | all damp | all common")
            continue
        try:
            target, step = parse_cli_command(command)
            await manager.dispatch(target, step, source="interactive", wait=False)
        except Exception as exc:
            logging.error("Command failed: %s", exc)


def build_parser() -> argparse.ArgumentParser:
    default_config = Path(__file__).with_name("scenario_config.sample.json")
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=Path, default=default_config)
    parser.add_argument("--backend", choices=["python", "rust"], default=None)
    return parser


async def async_main() -> None:
    args = build_parser().parse_args()
    try:
        config = load_config(args.config)
    except (OSError, ValueError, ValidationError, RuntimeError, json.JSONDecodeError) as exc:
        raise SystemExit(f"Failed to load scenario config: {exc}") from exc
    manager = ScenarioManager(config, args.backend)
    try:
        await manager.connect_all()
        await manager.run_startup_sequence()
        await interactive_loop(manager)
    finally:
        await manager.close()


class TeleopManager:
    """Terminal-based teleoperation for multiple robots across teams."""

    MOVE_MAP = {
        # Team A (wasd, qe)
        "w": ("teamA", 0.0, 1.2, 0.0),   # Forward
        "s": ("teamA", 0.0, -0.7, 0.0),  # Backward
        "a": ("teamA", -0.8, 0.0, 0.0),  # Left
        "d": ("teamA", 0.8, 0.0, 0.0),   # Right
        "q": ("teamA", 0.0, 0.0, -0.7),  # Yaw Left
        "e": ("teamA", 0.0, 0.0, 0.7),   # Yaw Right

        # Team B (ijkl, uo)
        "i": ("teamB", 0.0, 1.2, 0.0),   # Forward
        "k": ("teamB", 0.0, -0.7, 0.0),  # Backward
        "j": ("teamB", -0.8, 0.0, 0.0),  # Left
        "l": ("teamB", 0.8, 0.0, 0.0),   # Right
        "u": ("teamB", 0.0, 0.0, -0.7),  # Yaw Left
        "o": ("teamB", 0.0, 0.0, 0.7),   # Yaw Right
    }

    def __init__(self, manager: ScenarioManager) -> None:
        self.manager = manager
        self.running = False
        self.last_key_time: dict[str, float] = {}
        self.key_timeout = 0.4  # Seconds after which we assume key is released (increased for simultaneous presses)

    async def run(self) -> None:
        import select
        import termios
        import tty
        import os

        fd = sys.stdin.fileno()
        old_settings = termios.tcgetattr(fd)
        try:
            tty.setraw(fd)
            self.running = True
            print("\r\n--- TELEOP MODE ---")
            print("\r\nTeam A: WASD (move), QE (yaw)")
            print("\r\nTeam B: IJKL (move), UO (yaw)")
            print("\r\nActions (Shift + Number): ! (liedown), @ (standup), # (stop), $ (damp), % (red), ^ (common)")
            print("\r\nPress ESC or 'x' to exit.")

            # Command loop task
            cmd_task = asyncio.create_task(self.command_loop())

            action_map = {
                "!": "standup",  # Shift + 2
                "@": "liedown",  # Shift + 1
                "(": "stop",     # Shift + 3
                ")": "damp",     # Shift + 4
                "R": "red",      # Shift + 5
                "C": "common",   # Shift + 6
            }

            while self.running:
                # Non-blocking read from stdin
                rlist, _, _ = select.select([sys.stdin], [], [], 0.01)
                if rlist:
                    try:
                        data = os.read(fd, 1024)
                        for b in data:
                            raw_char = chr(b)
                            char = raw_char.lower()
                            
                            if char in {"\x1b", "x"}:  # ESC or 'x'
                                self.running = False
                                break
                            
                            if char in self.MOVE_MAP:
                                self.last_key_time[char] = asyncio.get_running_loop().time()
                            elif raw_char in action_map:
                                step = action_map[raw_char]
                                asyncio.create_task(self.manager.dispatch("all", step, source="interactive", wait=False))
                    except OSError:
                        pass

                await asyncio.sleep(0.01)

            await cmd_task

        finally:
            termios.tcsetattr(fd, termios.TCSADRAIN, old_settings)
            print("\r\nExited Teleop Mode.")

    async def command_loop(self) -> None:
        while self.running:
            now = asyncio.get_running_loop().time()
            velocities: dict[str, list[float]] = {
                "teamA": [0.0, 0.0, 0.0],  # lx, ly, rx
                "teamB": [0.0, 0.0, 0.0],
            }

            # Aggregate velocities from active keys
            for char, last_time in list(self.last_key_time.items()):
                if now - last_time < self.key_timeout:
                    team, lx, ly, rx = self.MOVE_MAP[char]
                    velocities[team][0] += lx
                    velocities[team][1] += ly
                    velocities[team][2] += rx
                else:
                    del self.last_key_time[char]

            # Clip velocities to [-1.0, 1.0]
            for team in velocities:
                velocities[team] = [max(-1.0, min(1.0, v)) for v in velocities[team]]

            # Send to robots
            for team, v in velocities.items():
                try:
                    robot_ids = self.manager.resolve_target(team)
                    for rid in robot_ids:
                        session = self.manager.sessions[rid]
                        if session.connected:
                            session.send_wireless_controller(lx=v[0], ly=v[1], rx=v[2])
                except Exception:
                    pass

            await asyncio.sleep(0.05)  # 20Hz update


if __name__ == "__main__":
    try:
        asyncio.run(async_main())
    except KeyboardInterrupt:
        pass
