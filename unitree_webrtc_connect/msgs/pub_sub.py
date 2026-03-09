import asyncio
import json
import time
from concurrent.futures import ThreadPoolExecutor

from ..constants import DATA_CHANNEL_TYPE
from .future_resolver import FutureResolver
from ..util import get_nested_field

class WebRTCDataChannelPubSub:
    def __init__(self, channel, n_workers: int = 4):
        self.channel = channel

        self.future_resolver = FutureResolver()
        self.subscriptions = {}  # Dictionary to hold callbacks keyed by topic
        # Use a thread pool to run subscription callbacks so they don't block the event loop
        self.callback_pool = ThreadPoolExecutor(max_workers=n_workers, thread_name_prefix="webrtc_sub_cb")

    def run_resolve(self, message):
        self.future_resolver.run_resolve_for_topic(message)

         # Extract the topic from the message
        topic = message.get("topic")
        if topic in self.subscriptions:
            # Execute the callback in a separate thread to prevent blocking
            # the WebRTC event loop with heavy tasks like pointcloud decoding.
            callback = self.subscriptions[topic]
            self.callback_pool.submit(callback, message)

    async def publish(self, topic, data=None, msg_type=None):
        channel = self.channel
        future = asyncio.get_event_loop().create_future()

        if channel.readyState == "open":
            message_dict = {
                "type": msg_type or DATA_CHANNEL_TYPE["MSG"],
                "topic": topic
            }
            # Only include "data" if it's not None
            if data is not None:
                message_dict["data"] = data
            
            # Convert the dictionary to a JSON string
            message = json.dumps(message_dict)

            channel.send(message)

            # Log the message being published
            logging.info(f"> message sent: {message}")

            # Store the future so it can be completed when the response is received
            uuid = (
                get_nested_field(data, "uuid") or
                get_nested_field(data, "header", "identity", "id") or 
                get_nested_field(data, "req_uuid")
            )

            self.future_resolver.save_resolve(msg_type or DATA_CHANNEL_TYPE["MSG"], topic, future, uuid)
        else:
            future.set_exception(Exception("Data channel is not open"))

        return await future
    

    def publish_without_callback(self, topic, data=None, msg_type=None):

        if self.channel.readyState == "open":
            message_dict = {
                "type": msg_type or DATA_CHANNEL_TYPE["MSG"],
                "topic": topic
            }

            # Only include "data" if it's not None
            if data is not None:
                message_dict["data"] = data
            
            # Convert the dictionary to a JSON string
            message = json.dumps(message_dict)
                
            self.channel.send(message)

            # Log the message being published
            logging.info(f"> message sent: {message}")
        else:
            Exception("Data channel is not open")
        

    async def publish_request_new(self, topic, options=None):
        # Generate a unique identifier
        generated_id = int(time.time() * 1000) % 2147483648 + random.randint(0, 1000)
        
        # Check if api_id is provided
        if not (options and "api_id" in options):
            print("Error: Please provide app id")
            return asyncio.Future().set_exception(Exception("Please provide app id"))

        # Build the request header and parameter
        request_payload = {
            "header": {
                "identity": {
                    "id": options.get("id", generated_id),
                    "api_id": options.get("api_id", 0)
                }
            },
            "parameter": ""
        }

        # Add data to parameter
        if options and "parameter" in options:
            request_payload["parameter"] = options["parameter"] if isinstance(options["parameter"], str) else json.dumps(options["parameter"])

        # Add priority if specified
        if options and "priority" in options:
            request_payload["header"]["policy"] = {
                "priority": 1
            }

        # Publish the request
        return await self.publish(topic, request_payload, DATA_CHANNEL_TYPE["REQUEST"])
    
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

    