import asyncio
import logging
from unitree_webrtc_connect.webrtc_driver import UnitreeWebRTCConnection, WebRTCConnectionMethod
from unitree_webrtc_connect.constants import RTC_TOPIC, SPORT_CMD
# 로그 레벨을 INFO로 설정해서 연결 과정 및 Heartbeat 상황을 확인

logging.basicConfig(level=logging.INFO)

async def test_reconnect(robot_ip="10.2.81.19"):
    print(f"[{robot_ip}] 로봇에 연결을 시도합니다...")
    
    # 1. 로봇과 연결 (로컬 IP 방식)
    conn = UnitreeWebRTCConnection(WebRTCConnectionMethod.LocalSTA, ip=robot_ip)
    await conn.connect()
    
    # 초기 연결 후 잠시 대기
    print("연결 완료. 5초 대기...")
    await asyncio.sleep(5)
    
    # ----------------------------------------------------------------------------------
    print("\n[TEST 1] Timeout 테스트: 비정상 토픽/API 호출 시 5초 후 Timeout 발생 여부 확인")
    try:
        await conn.datachannel.pub_sub.publish_request_new(
            "rt/nonexistent_topic", 
            {"api_id": 999999},
            timeout=5.0
        )
        print("❌ 실패: Timeout이 발생하지 않고 응답이 왔습니다.")
    except asyncio.TimeoutError:
        print("✅ 성공: 5초 대기 후 TimeoutError가 성공적으로 발생했습니다!")
    
    # ----------------------------------------------------------------------------------
    print("\n[TEST 2] Heartbeat & 자동 재연결 테스트: 로봇으로의 데이터 전송 강제 차단")
    
    # 통신 드롭을 시뮬레이션하기 위해 기존 datachannel의 send 함수 백업
    original_send = conn.datachannel.channel.send
    
    def blocked_send(data):
        print(f"🛑 [MOCK DROP] 데이터 전송 차단됨 (크기: {len(data)} bytes)")
        # 실질적으로 데이터를 보내지 않아 연결이 끊긴 것과 같은 효과 (Heartbeat 응답 없음)
        pass
    # send 함수를 가짜(mock) 함수로 바꿔버림
    conn.datachannel.channel.send = blocked_send
    # Heartbeat timeout 이 6.0초(3회분) 로 설정되어 있으므로, 약 7~8초 뒤에 _auto_reconnect 트리거 예상
    print("Heartbeat 실패를 감지할 때까지 약 10초 대기 중...")
    await asyncio.sleep(10)
    
    # 실제 환경의 경우 _auto_reconnect() 내부에서 disconnect() 후 connect()를 새롭게 생성하므로
    # 새로운 채널이 만들어지기 때문에 굳이 original_send를 복구할 필요가 없습니다.
    print("\n--- 통신 차단 해제 (새로운 커넥션으로 대체됨) ---")
    
    # 자동 재연결이 완료되고 RecoveryStand 명령까지 보내질 수 있도록 여유 시간 대기
    print("재연결 및 RecoveryStand 처리 대기 (10초)...")
    await asyncio.sleep(10)
    # ----------------------------------------------------------------------------------
    print("\n[TEST 3] 명령 연속성 테스트: 재연결 후 다시 명령이 잘 동작하는지 확인")
    try:
        print("'Hello' 모션 전송...")
        await conn.datachannel.pub_sub.publish_request_new(
            RTC_TOPIC["SPORT_MOD"], 
            {"api_id": SPORT_CMD["Hello"]},
            timeout=5.0
        )
        print("✅ 성공: 연결 단절 시뮬레이션 및 재연결 후 명령이 정상적으로 발송되었습니다!")
    except asyncio.TimeoutError:
        print("❌ 실패: 재연결 후에도 명령이 Timeout 되었습니다!")
        
    # 마무리
    await asyncio.sleep(5)
    await conn.disconnect()
    print("모든 테스트가 완료되었습니다.")
if __name__ == "__main__":
    asyncio.run(test_reconnect("10.2.81.19"))