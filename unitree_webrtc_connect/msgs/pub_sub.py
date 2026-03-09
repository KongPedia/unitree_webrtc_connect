import asyncio
import json
import logging
import random
import time
from concurrent.futures import ThreadPoolExecutor

from ..constants import DATA_CHANNEL_TYPE
from ..util import get_nested_field
from .future_resolver import FutureResolver


class WebRTCDataChannelPubSub:
    def __init__(self, channel, n_workers: int = 2, lidar_hz=15.0, video_fps=15.0):
        self.channel = channel

        self.future_resolver = FutureResolver()
        self.subscriptions = {}  # Dictionary to hold callbacks keyed by topic
        # Thread pool for binary message decoding (lidar, video) and subscription callbacks.
        # These are fire-and-forget: they never block the WebRTC event loop.
        # On Jetson Orin Nano (8-core): 8 workers handles concurrent lidar/video/odom streams.
        self.callback_pool = ThreadPoolExecutor(max_workers=n_workers, thread_name_prefix="webrtc_sub_cb")

        # Pre-decoding throttle logic
        self._last_process_time = {}
        self.throttle_limits = {
            "rt/utlidar/voxel_map_compressed": 1.0 / lidar_hz if lidar_hz > 0 else 0,
            "vid": 1.0 / video_fps if video_fps > 0 else 0,
        }

    def should_process(self, topic_or_type: str) -> bool:
        """Determines if a frame should be processed based on throttle limits."""
        if not topic_or_type or topic_or_type not in self.throttle_limits:
            return True

        now = time.time()
        last_time = self._last_process_time.get(topic_or_type, 0)
        limit = self.throttle_limits[topic_or_type]

        if now - last_time >= limit:
            self._last_process_time[topic_or_type] = now
            return True
        return False

    def run_resolve(self, message):
        self.future_resolver.run_resolve_for_topic(message)

        # Extract the topic from the message
        topic = message.get("topic")
        if topic in self.subscriptions:
            # Execute the callback in a separate thread to prevent blocking
            # the WebRTC event loop with heavy tasks like pointcloud decoding.
            callback = self.subscriptions[topic]
            self.callback_pool.submit(callback, message)

    async def publish(self, topic, data=None, msg_type=None, timeout=10.0):
        channel = self.channel
        future = asyncio.get_event_loop().create_future()

        if channel and channel.readyState == "open":
            message_dict = {"type": msg_type or DATA_CHANNEL_TYPE["MSG"], "topic": topic}
            # Only include "data" if it's not None
            if data is not None:
                message_dict["data"] = data

            # Convert the dictionary to a JSON string
            message = json.dumps(message_dict)

            try:
                channel.send(message)
            except Exception as e:
                logging.error(f"Failed to send on data channel: {e}")
                future.set_exception(e)
                # If we timeout, we'll raise. If no timeout, we return future which has exception set.
                if timeout is not None:
                    raise e
                return await future

            # Log the message being published
            logging.info(f"> message sent: {message}")

            # Store the future so it can be completed when the response is received
            uuid = (
                get_nested_field(data, "uuid")
                or get_nested_field(data, "header", "identity", "id")
                or get_nested_field(data, "req_uuid")
            )

            self.future_resolver.save_resolve(msg_type or DATA_CHANNEL_TYPE["MSG"], topic, future, uuid)
        else:
            future.set_exception(Exception("Data channel is not open"))

        if timeout is not None:
            try:
                return await asyncio.wait_for(future, timeout=timeout)
            except asyncio.TimeoutError:
                logging.error(f"Publish timeout after {timeout}s for topic {topic}")
                # Try to clean up future in resolver if it times out
                raise
        else:
            return await future

    def publish_without_callback(self, topic, data=None, msg_type=None):
        if self.channel.readyState == "open":
            message_dict = {"type": msg_type or DATA_CHANNEL_TYPE["MSG"], "topic": topic}

            # Only include "data" if it's not None
            if data is not None:
                message_dict["data"] = data

            # Convert the dictionary to a JSON string
            message = json.dumps(message_dict)

            try:
                self.channel.send(message)
            except Exception as e:
                logging.error(f"Failed to send on data channel: {e}")

            # Log the message being published
            logging.info(f"> message sent: {message}")
        else:
            Exception("Data channel is not open")

    async def publish_request_new(self, topic, options=None, timeout=10.0):
        # Generate a unique identifier
        generated_id = int(time.time() * 1000) % 2147483648 + random.randint(0, 1000)

        # Check if api_id is provided
        if not (options and "api_id" in options):
            print("Error: Please provide app id")
            return asyncio.Future().set_exception(Exception("Please provide app id"))

        # Build the request header and parameter
        request_payload = {
            "header": {"identity": {"id": options.get("id", generated_id), "api_id": options.get("api_id", 0)}},
            "parameter": "",
        }

        # Add data to parameter
        if options and "parameter" in options:
            request_payload["parameter"] = (
                options["parameter"] if isinstance(options["parameter"], str) else json.dumps(options["parameter"])
            )

        # Add priority if specified
        if options and "priority" in options:
            request_payload["header"]["policy"] = {"priority": 1}

        # Publish the request
        return await self.publish(topic, request_payload, DATA_CHANNEL_TYPE["REQUEST"], timeout=timeout)

    def subscribe(self, topic, callback=None):
        channel = self.channel

        if not channel or channel.readyState != "open":
            print("Error: Data channel is not open")
            return

        # Register the callback for the topic
        if callback:
            self.subscriptions[topic] = callback

        self.publish_without_callback(topic=topic, msg_type=DATA_CHANNEL_TYPE["SUBSCRIBE"])

    def unsubscribe(self, topic):
        channel = self.channel

        if not channel or channel.readyState != "open":
            print("Error: Data channel is not open")
            return

        self.publish_without_callback(topic=topic, msg_type=DATA_CHANNEL_TYPE["UNSUBSCRIBE"])
