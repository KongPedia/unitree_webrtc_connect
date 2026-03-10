import asyncio
import logging
import threading
import time
import warnings
from queue import Queue

import cv2
import numpy as np
from aiortc import MediaStreamTrack

try:
    from unitree_webrtc_connect.webrtc_driver_rs import UnitreeWebRTCConnection
except ImportError:
    warnings.warn(
        "webrtc_driver_rs import failed. Falling back to python webrtc_driver.",
        RuntimeWarning,
    )
    from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection

from unitree_webrtc_connect.constants import WebRTCConnectionMethod


height, width = 720, 1280
img = np.zeros((height, width, 3), dtype=np.uint8)
cv2.imshow("Video", img)
cv2.waitKey(1)

logging.basicConfig(level=logging.FATAL)


def main():
    frame_queue = Queue()
    conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip="192.168.8.181")

    async def recv_camera_stream(track: MediaStreamTrack):
        while True:
            frame = await track.recv()
            bgr = frame.to_ndarray(format="bgr24")
            frame_queue.put(bgr)

    def run_asyncio_loop(loop):
        asyncio.set_event_loop(loop)

        async def setup():
            try:
                await conn.connect()
                conn.video.switchVideoChannel(True)
                conn.video.add_track_callback(recv_camera_stream)
            except Exception as exc:
                logging.error("Error in WebRTC connection: %s", exc)

        loop.run_until_complete(setup())
        loop.run_forever()

    loop = asyncio.new_event_loop()
    asyncio_thread = threading.Thread(target=run_asyncio_loop, args=(loop,))
    asyncio_thread.start()

    try:
        while True:
            if not frame_queue.empty():
                frame = frame_queue.get()
                cv2.imshow("Video", frame)
                if cv2.waitKey(1) & 0xFF == ord("q"):
                    break
            else:
                time.sleep(0.01)
    finally:
        cv2.destroyAllWindows()
        loop.call_soon_threadsafe(loop.stop)
        asyncio_thread.join()


if __name__ == "__main__":
    main()
