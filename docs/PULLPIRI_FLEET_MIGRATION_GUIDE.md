# Pullpiri-Fleet Management Integration Guide
## uProtocol 기반 시나리오 상태 대시보드 연동 완성 가이드

---

## 1. 프로젝트 개요

### 1.1 목표
- **Pullpiri**에서 시나리오 상태 정보를 **Fleet Management** 대시보드로 전송
- uProtocol/Zenoh를 통한 실시간 데이터 연동
- InfluxDB + Grafana 기반 시나리오 모니터링 대시보드 구축

### 1.2 완성된 아키텍처
```
[Pullpiri] ──uProtocol/Zenoh──→ [Fleet Management] ──→ [InfluxDB] ──→ [Grafana]
    ↓                                    ↓                  ↓           ↓
├─ settingsservice                  ├─ fms-consumer    ├─ demo bucket  ├─ Dashboard
│  └─ StatusPublisher              │  └─ PullpiriStatusListener       │
├─ apiserver (시나리오 등록)         └─ influx-client   └─ Measurements: └─ Panels:
└─ filtergateway                       └─ writer.rs       • pullpiri_status  • Stat
   └─ VSS Integration                                     • pullpiri_scenario • Table
                                                                            • Time Series
```

---

## 2. 핵심 구현 사항

### 2.1 Pullpiri 측 구현

#### A. settingsservice 시나리오 상태 Publisher
**파일**: `/pullpiri/src/server/settingsservice/src/uprotocol/publisher.rs`

**주요 기능**:
- etcd에서 등록된 시나리오 목록 조회
- 각 시나리오의 현재 상태 조회
- 5초마다 JSON 형태로 uProtocol publish

**구현된 구조체**:
```rust
#[derive(Serialize)]
struct ScenarioStatus {
    name: String,
    state: String,
}
```

**publish되는 JSON 형태**:
```json
{
  "vehicle_id": "pullpiri-vehicle-001",
  "timestamp": 1773108105715,
  "scenarios": [
    {"name": "vss-speed-test", "state": "idle"}
  ],
  "scenario_count": 1
}
```

#### B. VSS Integration (완성)
**설계 문서**: `/pullpiri/docs/VSS_INTEGRATION_DESIGN.md`

**구현된 컴포넌트**:
- `VssManager`: VSS 데이터 구독 관리
- `VssData`: DDS 호환 데이터 구조
- `VehicleManager`: VSS + DDS 통합 관리
- `FilterGatewayManager`: VSS 데이터 처리

**지원 조건 연산자**: eq, lt, le, ge, gt (모두 검증 완료)

### 2.2 Fleet Management 측 구현

#### A. fms-consumer 시나리오 파싱
**파일**: `/fleet-management/components/fms-consumer/src/pullpiri.rs`

**구현 내용**:
- pullpiri JSON 메시지에서 scenarios 배열 파싱
- InfluxDB 저장용 ScenarioStatus 객체 변환
- `#[serde(default)]`로 하위 호환성 보장

#### B. InfluxDB 저장 로직
**파일**: `/fleet-management/components/influx-client/src/writer.rs`

**저장 구조**:
- **Measurement**: `pullpiri_scenario`
- **Tag**: `vehicleId`
- **Fields**: 
  - `scenarioName`: 시나리오 이름
  - `scenarioState`: 현재 상태 (idle/waiting/satisfied/etc.)
  - `triggerCount`: 트리거 횟수
  - `createdDateTime`: 타임스탬프

---

## 3. 배포 및 실행 순서

### 3.1 올바른 실행 순서 (중요!)

#### 1단계: Fleet Management 먼저 실행
```bash
cd /home/auto/projects/edo/fleet-management
docker compose -f fms-blueprint-compose.yaml -f fms-blueprint-compose-zenoh.yaml -f docker-compose.override.yaml up -d
```

**확인사항**:
- Zenoh router가 7447 포트에서 listening 중인지 확인
```bash
netstat -tuln | grep 7447
# 출력: tcp 0 0 0.0.0.0:7447 0.0.0.0:* LISTEN
```

#### 2단계: Pullpiri 실행
```bash
cd /home/auto/projects/edo/pullpiri
sudo make dev-install
```

**확인사항**:
- settingsservice가 Zenoh에 연결 성공
- uProtocol status가 5초마다 publish 중

### 3.2 시나리오 등록
```bash
curl -X POST http://192.168.10.2:47099/api/artifact \
  -H "Content-Type: application/x-yaml" \
  --data-binary @/home/auto/projects/edo/pullpiri/examples/resources/vss_scenario.yaml
```

---

## 4. 검증 및 모니터링

### 4.1 데이터 흐름 검증

#### A. Pullpiri 로그 확인
```bash
sudo podman logs piccolo-settingsservice | tail -5
```
**기대 결과**:
```
Publishing pullpiri status JSON: {"scenario_count":1,"scenarios":[{"name":"vss-speed-test","state":"idle"}],"timestamp":1773108105715,"vehicle_id":"pullpiri-vehicle-001"}
DEBUG uProtocol status published successfully
```

#### B. InfluxDB 데이터 확인
```bash
docker exec influxDB influx query 'from(bucket: "demo") |> range(start: -5m) |> filter(fn: (r) => r._measurement == "pullpiri_scenario")' --org sdv
```

**기대 결과**:
```
Table: pullpiri_scenario
vehicleId: pullpiri-vehicle-001
scenarioName: vss-speed-test
scenarioState: idle
triggerCount: 0
```

### 4.2 Grafana 대시보드 쿼리

#### A. 활성 시나리오 수 (Stat Panel)
```flux
from(bucket: "demo")
  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
  |> filter(fn: (r) => r._measurement == "pullpiri_status")
  |> filter(fn: (r) => r._field == "active")
  |> last()
  |> count()
```

#### B. 시나리오 상태 목록 (Table Panel)
```flux
from(bucket: "demo")
  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
  |> filter(fn: (r) => r._measurement == "pullpiri_scenario")
  |> filter(fn: (r) => r._field == "scenarioName" or r._field == "scenarioState")
  |> group(columns: ["vehicleId", "_time"])
  |> pivot(rowKey: ["_time", "vehicleId"], columnKey: ["_field"], valueColumn: "_value")
  |> group()
  |> sort(columns: ["_time"], desc: true)
```

#### C. 시나리오 상태 히스토리 (Time Series)
```flux
from(bucket: "demo")
  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
  |> filter(fn: (r) => r._measurement == "pullpiri_scenario")
  |> filter(fn: (r) => r._field == "scenarioState")
  |> group(columns: ["vehicleId", "scenarioName"])
```

---

## 5. 트러블슈팅

### 5.1 일반적인 문제들

#### A. Zenoh 연결 실패
**증상**: settingsservice 로그에 "Connection refused" 에러
```
WARN Unable to connect to tcp/localhost:7447! Connection refused
```

**해결방법**:
1. Fleet Management가 먼저 실행되었는지 확인
2. 7447 포트가 바인딩되었는지 확인: `netstat -tuln | grep 7447`
3. `docker-compose.override.yaml`에 포트 설정이 있는지 확인

#### B. 시나리오 데이터가 InfluxDB에 저장되지 않음
**원인 분석 순서**:
1. settingsservice가 JSON을 publish하고 있는지 확인
2. fms-consumer가 메시지를 수신하고 있는지 확인
3. fms-consumer가 재빌드되었는지 확인

#### C. Grafana에서 데이터가 보이지 않음
**확인사항**:
1. InfluxDB에서 직접 쿼리 실행 확인
2. Grafana의 시간 범위 설정 확인
3. measurement 이름이 정확한지 확인 (`pullpiri_scenario`)

### 5.2 포트 설정 문제

docker-compose.override.yaml에 Zenoh 포트 설정이 필요:
```yaml
services:
  fms-zenoh-router:
    ports:
      - "0.0.0.0:7447:7447"
```

---

## 6. 주요 달성 결과

### 6.1 ✅ 완료된 기능들
- [x] **uProtocol 통신**: Pullpiri → Fleet Management
- [x] **시나리오 상태 전송**: etcd에서 조회 → JSON publish
- [x] **InfluxDB 저장**: pullpiri_scenario measurement
- [x] **VSS 통합**: Vehicle Signal 기반 시나리오 처리
- [x] **조건 평가**: eq, lt, le, ge, gt 모든 연산자 지원
- [x] **실시간 모니터링**: 5초 간격 상태 업데이트
- [x] **대시보드 연동**: Grafana 쿼리 제공

### 6.2 📊 데이터 구조
**pullpiri_status measurement**:
- vehicle_id (tag), active, timestamp (fields)

**pullpiri_scenario measurement**:
- vehicleId (tag)
- scenarioName, scenarioState, triggerCount, createdDateTime (fields)

### 6.3 🔄 데이터 흐름
```
시나리오 등록 → etcd 저장 → settingsservice 조회 → JSON 생성 → 
uProtocol publish → Zenoh 전송 → fms-consumer 수신 → InfluxDB 저장 → 
Grafana 시각화
```

---

## 7. 확장 가능성

### 7.1 추가 가능한 기능들
- 시나리오 성공/실패 통계
- 시나리오 실행 시간 측정
- 다중 vehicle 지원
- 알람 및 알림 기능
- 시나리오 성능 분석

### 7.2 스케일링 고려사항
- 다수의 pullpiri 인스턴스 지원
- InfluxDB 샤딩/클러스터링
- Grafana 대시보드 템플릿화
- 실시간 알림 시스템

---

## 8. 참고 자료

### 8.1 관련 문서
- `/pullpiri/docs/VSS_INTEGRATION_DESIGN.md`: VSS 통합 설계
- `/fleet-management/README.md`: Fleet Management 개요
- Grafana Flux 쿼리 문법

### 8.2 주요 파일들
**Pullpiri**:
- `src/server/settingsservice/src/uprotocol/publisher.rs`
- `src/player/filtergateway/src/vehicle/vss/mod.rs`
- `examples/resources/vss_scenario.yaml`

**Fleet Management**:
- `components/fms-consumer/src/pullpiri.rs`
- `components/influx-client/src/writer.rs`
- `docker-compose.override.yaml`

---

**문서 버전**: 1.0  
**작성일**: 2026-03-10  
**상태**: 완성 및 검증 완료 ✅  
**담당자**: GitHub Copilot & leeeunkoo  

---

**🎉 프로젝트 완성!**  
Pullpiri와 Fleet Management 간의 uProtocol 기반 시나리오 상태 연동이 성공적으로 완료되었습니다. 이제 실시간으로 시나리오 상태를 대시보드에서 모니터링할 수 있습니다!