import asyncio
import logging
import time

from ..constants import DATA_CHANNEL_TYPE


class WebRTCDataChannelHeartBeat:
    def __init__(self, channel, pub_sub):
        self.channel = channel
        self.heartbeat_timer = None
        self.heartbeat_response = None
        self.publish = pub_sub.publish_without_callback
        self.on_connection_lost = None
        self.timeout_threshold = 6.0  # 6 seconds timeout (3 missed heartbeats)

    def set_on_connection_lost_callback(self, callback):
        """Register a callback to be called upon connection loss detected via heartbeat."""
        if callback and callable(callback):
            self.on_connection_lost = callback

    def _format_date(self, timestamp):
        return time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(timestamp))

    def start_heartbeat(self):
        """Start sending heartbeat messages every 2 seconds."""
        # Reset heartbeat response time on start
        self.heartbeat_response = time.time()
        self.heartbeat_timer = asyncio.get_event_loop().call_later(2, self.send_heartbeat)

    def stop_heartbeat(self):
        """Stop the heartbeat."""
        if self.heartbeat_timer:
            self.heartbeat_timer.cancel()
            self.heartbeat_timer = None

    def send_heartbeat(self):
        """Send a heartbeat message."""
        current_time = time.time()

        # Check if heartbeat timed out
        if self.heartbeat_response and (current_time - self.heartbeat_response > self.timeout_threshold):
            logging.error(
                f"Heartbeat timeout detected. Last response was {current_time - self.heartbeat_response:.1f}s ago."
            )
            if self.on_connection_lost:
                self.on_connection_lost()
            # If we lost connection, stop sending heartbeats until restarted
            self.stop_heartbeat()
            return

        if self.channel.readyState == "open":
            formatted_time = self._format_date(current_time)
            data = {"timeInStr": formatted_time, "timeInNum": int(current_time)}
            self.publish(
                "",
                data,
                DATA_CHANNEL_TYPE["HEARTBEAT"],
            )
        # Schedule the next heartbeat
        self.heartbeat_timer = asyncio.get_event_loop().call_later(2, self.send_heartbeat)

    def handle_response(self, message):
        """Handle a received heartbeat message."""
        self.heartbeat_response = time.time()
        logging.info("Heartbeat response received.")
