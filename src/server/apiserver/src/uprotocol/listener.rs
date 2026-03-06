// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! uProtocol 메시지 리스너
//!
//! Cloud에서 수신한 시나리오를 파싱하고 manager로 전달

use tokio::sync::mpsc::Sender;

use up_rust::{UListener, UMessage};

use common::logd;

/// Cloud에서 전송하는 시나리오 다운로드 메시지
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ScenarioDownload {
    /// 시나리오 고유 ID
    pub id: String,
    /// 시나리오 이름
    pub name: String,
    /// YAML 형식의 시나리오 내용
    pub yaml_content: String,
    /// 수행할 액션 (DEPLOY, UPDATE, DELETE)
    pub action: String,
}

/// 시나리오 다운로드 리스너
///
/// uProtocol 메시지를 수신하여 manager로 전달
pub struct ScenarioListener {
    /// 시나리오를 manager로 전달할 채널
    scenario_sender: Sender<ScenarioDownload>,
}

impl ScenarioListener {
    /// 새 리스너 생성
    pub fn new(scenario_sender: Sender<ScenarioDownload>) -> Self {
        Self { scenario_sender }
    }
}

#[async_trait::async_trait]
impl UListener for ScenarioListener {
    async fn on_receive(&self, msg: UMessage) {
        logd!(1, "Received uProtocol message");

        // Extract payload from uProtocol message
        let payload = match msg.payload {
            Some(bytes) => bytes,
            None => {
                logd!(4, "Received uProtocol message with empty payload");
                return;
            }
        };

        // Parse JSON payload to ScenarioDownload
        match serde_json::from_slice::<ScenarioDownload>(&payload) {
            Ok(scenario_download) => {
                logd!(
                    2,
                    "Received scenario from Cloud: {} (action: {})",
                    scenario_download.name,
                    scenario_download.action
                );

                if let Err(e) = self.scenario_sender.send(scenario_download).await {
                    logd!(4, "Failed to forward scenario to manager: {}", e);
                }
            }
            Err(e) => {
                logd!(4, "Failed to parse ScenarioDownload JSON: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    /// Test ScenarioDownload deserialization
    #[test]
    fn test_scenario_download_deserialize() {
        let json = r#"{
            "id": "test-id-123",
            "name": "test-scenario",
            "yaml_content": "apiVersion: v1\nkind: Scenario",
            "action": "DEPLOY"
        }"#;

        let result: Result<ScenarioDownload, _> = serde_json::from_str(json);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert_eq!(scenario.id, "test-id-123");
        assert_eq!(scenario.name, "test-scenario");
        assert_eq!(scenario.action, "DEPLOY");
    }

    /// Test ScenarioListener can be created
    #[tokio::test]
    async fn test_scenario_listener_creation() {
        let (tx, _rx) = mpsc::channel::<ScenarioDownload>(10);
        let listener = ScenarioListener::new(tx);
        // Just verify it compiles and can be created
        assert!(true);
        drop(listener);
    }

    /// Test ScenarioDownload serialization
    #[test]
    fn test_scenario_download_serialize() {
        let scenario = ScenarioDownload {
            id: "test-id".to_string(),
            name: "test-name".to_string(),
            yaml_content: "test-content".to_string(),
            action: "DEPLOY".to_string(),
        };

        let json = serde_json::to_string(&scenario);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("test-id"));
        assert!(json_str.contains("DEPLOY"));
    }
}
