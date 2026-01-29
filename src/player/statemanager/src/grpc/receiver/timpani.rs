/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::external::timpani::fault_service_server::FaultService;
use common::external::timpani::{FaultInfo, Response as TimpaniResponse};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct TimpaniReceiver {}

#[tonic::async_trait]
impl FaultService for TimpaniReceiver {
    async fn notify_fault(
        &self,
        info: Request<FaultInfo>,
    ) -> Result<Response<TimpaniResponse>, Status> {
        let info = info.into_inner();
        println!("Received fault notification: {:?}", info);

        // Process the fault information and generate a response
        let response = TimpaniResponse { status: 0 };
        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_notify_fault_returns_success() {
        let receiver = TimpaniReceiver::default();

        // Use default FaultInfo (prost types implement Default)
        let info = FaultInfo::default();

        let req = Request::new(info);
        let resp = receiver.notify_fault(req).await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.get_ref().status, 0);
    }

    #[tokio::test]
    async fn test_notify_fault_concurrent_calls() {
        let receiver = TimpaniReceiver::default();

        // Spawn multiple concurrent notify_fault calls to ensure no panics and consistent responses
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(tokio::spawn(async move {
                let r = TimpaniReceiver::default();
                let info = FaultInfo::default();
                let req = Request::new(info);
                let res = r.notify_fault(req).await;
                res
            }));
        }

        for h in handles {
            let res = h.await.expect("task panicked");
            assert!(res.is_ok());
            let out = res.unwrap();
            assert_eq!(out.get_ref().status, 0);
        }
    }
}
