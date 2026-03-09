# VSS Integration Design Document
## FilterGateway VSS 지원 추가 설계

---

## 1. 현재 DDS 아키텍처 분석

### 1.1 DDS 데이터 흐름
```
DdsManager → create_typed_listener() → DdsTopicListener
    ↓
tx: Sender<DdsData>
    ↓
rx_dds: Receiver<DdsData> (FilterGatewayManager)
    ↓
process_dds_data() → Filter::process_data()
    ↓
Filter::meet_scenario_condition() → ActionController
```

### 1.2 핵심 컴포넌트

#### A. VehicleManager (`vehicle/mod.rs`)
- **역할**: DDS 데이터 구독 관리
- **주요 메서드**:
  - `new(tx: Sender<DdsData>)`: DdsManager에 전달할 채널 생성
  - `init()`: DDS 시스템 초기화
  - `subscribe_topic(topic_name, data_type_name)`: 특정 토픽 구독
  - `unsubscribe_topic(topic_name)`: 구독 해제

#### B. DdsManager (`vehicle/dds/mod.rs`)
- **역할**: DDS 리스너 생성 및 관리
- **주요 필드**:
  - `listeners: HashMap<String, Box<dyn DdsTopicListener>>`
  - `tx: Sender<DdsData>`
  - `domain_id: i32`
- **주요 메서드**:
  - `create_typed_listener(topic_name, data_type_name)`: 타입별 리스너 생성
  - `remove_listener(topic_name)`: 리스너 제거

#### C. DdsData 구조체
```rust
pub struct DdsData {
    pub name: String,        // 토픽 이름
    pub value: String,       // 전체 값 (문자열)
    pub fields: HashMap<String, String>,  // 필드별 값
}
```

#### D. FilterGatewayManager (`manager.rs`)
- **주요 필드**:
  - `rx_grpc: Receiver<ScenarioParameter>`: 시나리오 등록 요청 수신
  - `rx_dds: Receiver<DdsData>`: DDS 데이터 수신
  - `filters: Vec<Filter>`: 활성화된 필터 목록
  - `vehicle_manager: VehicleManager`: DDS 구독 관리
- **주요 메서드**:
  - `process_grpc_requests()`: 시나리오 등록 처리
  - `process_dds_data()`: DDS 데이터를 필터에 전달

#### E. Filter (`filter/mod.rs`)
- **역할**: 시나리오 조건 평가
- **주요 메서드**:
  - `process_data(dds_data: &DdsData)`: 데이터 처리
  - `meet_scenario_condition(data: &DdsData)`: 조건 평가
  - `evaluate_condition()`: 표현식 평가 (eq, gt, lt, etc.)

---

## 2. VSS 아키텍처 설계

### 2.1 목표 VSS 데이터 흐름 (DDS와 동일 구조)
```
VssManager → create_vss_subscription() → VssSubscriber
    ↓
tx: Sender<VssData>
    ↓
rx_vss: Receiver<VssData> (FilterGatewayManager)
    ↓
process_vss_data() → Filter::process_data()
    ↓
Filter::meet_scenario_condition() → ActionController
```

### 2.2 신규 컴포넌트 설계

#### A. VssData 구조체 (신규 생성)
**위치**: `vehicle/vss/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VssData {
    pub name: String,        // VSS 경로 (예: "Vehicle.Speed")
    pub value: String,       // 값 (문자열 변환)
    pub fields: HashMap<String, String>,  // 호환성을 위한 필드
}

impl From<VssTrigger> for VssData {
    fn from(trigger: VssTrigger) -> Self {
        let value_string = match trigger.value {
            VssValue::String(s) => s,
            VssValue::Bool(b) => b.to_string(),
            VssValue::Int32(i) => i.to_string(),
            VssValue::Int64(i) => i.to_string(),
            VssValue::Uint32(u) => u.to_string(),
            VssValue::Uint64(u) => u.to_string(),
            VssValue::Float(f) => f.to_string(),
            VssValue::Double(d) => d.to_string(),
        };
        
        let mut fields = HashMap::new();
        fields.insert("value".to_string(), value_string.clone());
        
        VssData {
            name: trigger.path,
            value: value_string,
            fields,
        }
    }
}
```

#### B. VssManager 구조체 (신규 생성)
**위치**: `vehicle/vss/mod.rs`

```rust
pub struct VssManager {
    /// VSS subscriber instance
    subscriber: Option<VssSubscriber>,
    /// Active subscriptions (VSS path → channel)
    subscriptions: HashMap<String, Sender<VssTrigger>>,
    /// Channel for sending VSS data
    tx: Sender<VssData>,
    /// Databroker URI
    databroker_uri: String,
}

impl VssManager {
    pub fn new(tx: Sender<VssData>, databroker_uri: String) -> Self {
        Self {
            subscriber: None,
            subscriptions: HashMap::new(),
            tx,
            databroker_uri,
        }
    }
    
    pub async fn init(&mut self) -> Result<()> {
        // VssSubscriber 생성
        let subscriber = VssSubscriber::new(&self.databroker_uri).await
            .map_err(|e| anyhow!("Failed to create VssSubscriber: {}", e))?;
        self.subscriber = Some(subscriber);
        Ok(())
    }
    
    pub async fn create_vss_subscription(&mut self, vss_path: String) -> Result<()> {
        // 이미 구독 중인지 확인
        if self.subscriptions.contains_key(&vss_path) {
            logd!(4, "Already subscribed to VSS path: {}", vss_path);
            return Ok(());
        }
        
        // VssTrigger를 VssData로 변환하는 채널 생성
        let (trigger_tx, mut trigger_rx) = mpsc::channel::<VssTrigger>(100);
        let data_tx = self.tx.clone();
        let path_clone = vss_path.clone();
        
        // 백그라운드 태스크: VssTrigger를 VssData로 변환
        tokio::spawn(async move {
            while let Some(trigger) = trigger_rx.recv().await {
                let vss_data = VssData::from(trigger);
                if let Err(e) = data_tx.send(vss_data).await {
                    logd!(5, "Failed to send VssData for {}: {}", path_clone, e);
                    break;
                }
            }
        });
        
        // VssSubscriber에 구독 요청
        if let Some(ref mut subscriber) = self.subscriber {
            subscriber.subscribe(vec![vss_path.clone()], trigger_tx).await
                .map_err(|e| anyhow!("Failed to subscribe to {}: {}", vss_path, e))?;
        }
        
        self.subscriptions.insert(vss_path, trigger_tx);
        Ok(())
    }
    
    pub async fn remove_subscription(&mut self, vss_path: &str) -> Result<()> {
        self.subscriptions.remove(vss_path);
        Ok(())
    }
}
```

#### C. VehicleManager 확장
**위치**: `vehicle/mod.rs`

```rust
pub struct VehicleManager {
    /// DDS Manager instance
    dds_manager: dds::DdsManager,
    /// VSS Manager instance (옵셔널 - vss feature flag)
    #[cfg(feature = "vss")]
    vss_manager: Option<vss::VssManager>,
}

impl VehicleManager {
    pub fn new(tx_dds: Sender<DdsData>, tx_vss: Sender<VssData>) -> Self {
        Self {
            dds_manager: dds::DdsManager::new(tx_dds),
            #[cfg(feature = "vss")]
            vss_manager: None,
        }
    }
    
    #[cfg(feature = "vss")]
    pub async fn init_vss(&mut self, databroker_uri: String, tx_vss: Sender<VssData>) -> Result<()> {
        let mut vss_manager = vss::VssManager::new(tx_vss, databroker_uri);
        vss_manager.init().await?;
        self.vss_manager = Some(vss_manager);
        Ok(())
    }
    
    #[cfg(feature = "vss")]
    pub async fn subscribe_vss_signal(&mut self, vss_path: String) -> Result<()> {
        if let Some(ref mut vss_manager) = self.vss_manager {
            vss_manager.create_vss_subscription(vss_path).await
        } else {
            Err(anyhow!("VSS Manager not initialized"))
        }
    }
    
    #[cfg(feature = "vss")]
    pub async fn unsubscribe_vss_signal(&mut self, vss_path: String) -> Result<()> {
        if let Some(ref mut vss_manager) = self.vss_manager {
            vss_manager.remove_subscription(&vss_path).await
        } else {
            Ok(())
        }
    }
}
```

#### D. FilterGatewayManager 확장
**위치**: `manager.rs`

```rust
pub struct FilterGatewayManager {
    /// Receiver for scenario information from gRPC
    pub rx_grpc: Arc<Mutex<mpsc::Receiver<ScenarioParameter>>>,
    /// Receiver for DDS data
    pub rx_dds: Arc<Mutex<mpsc::Receiver<DdsData>>>,
    /// Receiver for VSS data (신규 추가)
    #[cfg(feature = "vss")]
    pub rx_vss: Arc<Mutex<mpsc::Receiver<VssData>>>,
    /// Active filters for scenarios
    pub filters: Arc<Mutex<Vec<Filter>>>,
    /// gRPC sender for action controller
    pub sender: Arc<Mutex<FilterGatewaySender>>,
    /// Vehicle manager for handling vehicle data
    pub vehicle_manager: Arc<Mutex<VehicleManager>>,
}

impl FilterGatewayManager {
    pub async fn new(rx_grpc: mpsc::Receiver<ScenarioParameter>) -> Self {
        let (tx_dds, rx_dds) = mpsc::channel::<DdsData>(10);
        let (tx_vss, rx_vss) = mpsc::channel::<VssData>(10);
        
        let vehicle_manager = VehicleManager::new(tx_dds, tx_vss);
        
        // VSS 초기화
        #[cfg(feature = "vss")]
        if let Ok(databroker_uri) = std::env::var("KUKSA_DATABROKER_URI") {
            if let Err(e) = vehicle_manager.init_vss(databroker_uri, tx_vss).await {
                logd!(4, "Failed to initialize VSS: {:?}", e);
            }
        }
        
        Self {
            rx_grpc: Arc::new(Mutex::new(rx_grpc)),
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            #[cfg(feature = "vss")]
            rx_vss: Arc::new(Mutex::new(rx_vss)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender: Arc::new(Mutex::new(FilterGatewaySender::new())),
            vehicle_manager: Arc::new(Mutex::new(vehicle_manager)),
        }
    }
    
    /// VSS 데이터 처리 (DDS와 동일한 패턴)
    #[cfg(feature = "vss")]
    async fn process_vss_data(&self) -> Result<()> {
        let rx_vss = Arc::clone(&self.rx_vss);
        
        loop {
            let mut receiver = rx_vss.lock().await;
            
            match receiver.recv().await {
                Some(vss_data) => {
                    if !vss_data.name.is_empty() && !vss_data.value.is_empty() {
                        logd!(3, "Received VSS data: signal={}, value={}", vss_data.name, vss_data.value);
                    }
                    
                    // Forward data to all active filters
                    let mut filters = self.filters.lock().await;
                    for filter in filters.iter_mut() {
                        if filter.is_active() {
                            // VSS 데이터를 DDS 데이터로 변환하여 전달
                            let dds_data = DdsData {
                                name: vss_data.name.clone(),
                                value: vss_data.value.clone(),
                                fields: vss_data.fields.clone(),
                            };
                            
                            if let Err(e) = filter.process_data(&dds_data).await {
                                logd!(5, "Error processing VSS data in filter {}: {:?}", 
                                      filter.scenario_name, e);
                            }
                        }
                    }
                }
                None => {
                    logd!(5, "VSS data channel closed, stopping processor");
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn run(&self) -> Result<()> {
        tokio::select! {
            _ = self.process_grpc_requests() => {},
            _ = self.process_dds_data() => {},
            #[cfg(feature = "vss")]
            _ = self.process_vss_data() => {},
        }
        Ok(())
    }
}
```

#### E. 시나리오 등록 처리 수정
**위치**: `manager.rs` - `process_grpc_requests()` 메서드

```rust
match param.action {
    0 => {
        // Allow
        logd!(1, "📥 Scenario registered via uProtocol: {}", param.scenario.get_name());
        
        if let Some(condition) = param.scenario.get_conditions() {
            let operand_type = condition.get_operand_type();
            let operand_value = condition.get_operand_value();
            
            logd!(1, "   Condition - Type: {}, Signal: {}", operand_type, operand_value);
            
            #[cfg(feature = "vss")]
            if operand_type.to_uppercase() == "VSS" {
                logd!(1, "   ✅ VSS scenario detected - subscribing to: {}", operand_value);
                
                // VSS 신호 구독
                let mut vehicle_manager = self.vehicle_manager.lock().await;
                if let Err(e) = vehicle_manager.subscribe_vss_signal(operand_value).await {
                    logd!(5, "Error subscribing to VSS signal: {:?}", e);
                }
                
                // 필터 등록
                self.launch_scenario_filter(param.scenario).await?;
                continue;
            }
            
            // DDS 처리
            if operand_type.to_uppercase() == "DDS" {
                logd!(1, "   ℹ️  DDS scenario - proceeding with DDS subscription");
                
                let topic_name = operand_value.clone();
                let data_type_name = operand_value;
                
                let mut vehicle_manager = self.vehicle_manager.lock().await;
                if let Err(e) = vehicle_manager.subscribe_topic(topic_name, data_type_name).await {
                    logd!(5, "Error subscribing to DDS topic: {:?}", e);
                }
                
                self.launch_scenario_filter(param.scenario).await?;
            }
        }
    }
    1 => {
        // Withdraw
        if let Some(condition) = param.scenario.get_conditions() {
            let operand_type = condition.get_operand_type();
            let operand_value = condition.get_operand_value();
            
            #[cfg(feature = "vss")]
            if operand_type.to_uppercase() == "VSS" {
                let mut vehicle_manager = self.vehicle_manager.lock().await;
                if let Err(e) = vehicle_manager.unsubscribe_vss_signal(operand_value).await {
                    logd!(5, "Error unsubscribing from VSS signal: {:?}", e);
                }
            } else {
                let mut vehicle_manager = self.vehicle_manager.lock().await;
                if let Err(e) = vehicle_manager.unsubscribe_topic(param.scenario.get_name()).await {
                    logd!(5, "Error unsubscribing from DDS topic: {:?}", e);
                }
            }
        }
        
        self.remove_scenario_filter(param.scenario.get_name()).await?;
    }
}
```

---

## 3. 마이그레이션 단계

### Phase 1: 기본 구조 생성 ✅
1. `vehicle/vss/mod.rs` 생성
2. `VssData` 구조체 정의
3. `VssManager` 구조체 정의

### Phase 2: VehicleManager 통합
1. `VehicleManager`에 VSS 필드 추가
2. `init_vss()` 메서드 구현
3. `subscribe_vss_signal()` / `unsubscribe_vss_signal()` 구현

### Phase 3: FilterGatewayManager 통합
1. `rx_vss` 채널 추가
2. `process_vss_data()` 메서드 구현
3. `run()` 메서드에 `process_vss_data()` 추가
4. VSS 초기화 로직 추가

### Phase 4: 시나리오 처리 로직 수정
1. `process_grpc_requests()`에서 VSS/DDS 타입 분기
2. VSS 시나리오 등록 시 `subscribe_vss_signal()` 호출
3. VSS 시나리오 철회 시 `unsubscribe_vss_signal()` 호출

### Phase 5: 테스트 및 검증
1. VSS 시나리오 등록 테스트
2. VSS 데이터 수신 테스트
3. 필터 조건 평가 테스트
4. ActionController 트리거 테스트

---

## 4. 파일 수정 목록

### 신규 파일
- [ ] `src/player/filtergateway/src/vehicle/vss/mod.rs` (VssManager, VssData)

### 수정 파일
- [ ] `src/player/filtergateway/src/vehicle/mod.rs` (VehicleManager VSS 지원)
- [ ] `src/player/filtergateway/src/manager.rs` (FilterGatewayManager VSS 통합)
- [ ] `src/common/src/spec/artifact/scenario.rs` (이미 완료: get_operand_type())

---

## 5. 주요 고려사항

### 5.1 타입 변환
- **VssTrigger → VssData → DdsData**: 필터가 기존 `DdsData` 인터페이스를 사용하므로 변환 필요
- **VssValue 타입**: 다양한 타입 지원 (String, Bool, Int32, Int64, Uint32, Uint64, Float, Double)

### 5.2 채널 관리
- **DDS와 VSS 분리**: 각각 독립적인 채널 사용
- **백그라운드 태스크**: VssTrigger를 VssData로 변환하는 태스크 필요

### 5.3 Feature Flag
- `#[cfg(feature = "vss")]`를 사용하여 VSS 기능 활성화/비활성화
- VSS 미지원 환경에서도 컴파일 가능

### 5.4 에러 처리
- VssManager 초기화 실패 시 경고 로그 출력 후 계속 진행
- VSS 구독 실패 시 에러 로그 출력

### 5.5 동시성
- `VehicleManager`는 `Arc<Mutex<>>`로 공유
- VSS 구독은 비동기로 처리

---

## 6. 예상 동작 흐름

```
1. FilterGateway 시작
   ↓
2. VehicleManager::init_vss() 호출 (KUKSA_DATABROKER_URI 존재 시)
   ↓
3. VssManager::init() → VssSubscriber 생성
   ↓
4. VSS 시나리오 등록 (uProtocol via apiserver)
   ↓
5. process_grpc_requests() → operand_type == "VSS" 감지
   ↓
6. vehicle_manager.subscribe_vss_signal("Vehicle.Speed")
   ↓
7. VssManager::create_vss_subscription()
   ↓
8. VssSubscriber::subscribe() 호출
   ↓
9. Databroker에서 Vehicle.Speed 업데이트 수신
   ↓
10. VssTrigger → VssData 변환
    ↓
11. rx_vss 채널로 전송
    ↓
12. process_vss_data() 수신
    ↓
13. VssData → DdsData 변환
    ↓
14. Filter::process_data() 호출
    ↓
15. Filter::meet_scenario_condition() 평가
    ↓
16. 조건 만족 시 ActionController::trigger_action()
```

---

## 7. 다음 단계

1. **Phase 1 구현**: `vehicle/vss/mod.rs` 생성 및 기본 구조 작성
2. **Phase 2 구현**: `VehicleManager` VSS 통합
3. **Phase 3 구현**: `FilterGatewayManager` VSS 데이터 처리
4. **Phase 4 구현**: 시나리오 등록/철회 로직 수정
5. **빌드 및 테스트**: `sudo make dev-install` 실행 후 시나리오 등록 테스트
6. **검증**: VSS 데이터 수신 및 필터 동작 확인

---

## 8. 테스트 시나리오

### 테스트 1: VSS 시나리오 등록
```bash
curl -X POST http://192.168.10.2:47099/api/artifact \
  -H "Content-Type: application/x-yaml" \
  --data-binary @examples/resources/vss_scenario.yaml
```

**기대 결과**:
```
📥 Scenario registered via uProtocol: vss-speed-test
   Condition - Type: VSS, Signal: Vehicle.Speed
   ✅ VSS scenario detected - subscribing to: Vehicle.Speed
```

### 테스트 2: VSS 데이터 수신
**csv-provider 실행 중 확인**:
```bash
sudo podman logs piccolo-filtergateway | grep "Received VSS data"
```

**기대 결과**:
```
Received VSS data: signal=Vehicle.Speed, value=46.8984375
```

### 테스트 3: 필터 조건 평가
**Vehicle.Speed > 50 조건 만족 시**:
```
Checking condition for scenario: vss-speed-test
Scenario condition met! Triggering action for: vss-speed-test
```

---

**문서 버전**: 1.0
**작성일**: 2026-03-09
**상태**: 설계 완료, 구현 대기
