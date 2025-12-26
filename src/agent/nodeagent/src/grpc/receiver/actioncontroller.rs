/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::nodeagent::fromactioncontroller::{HandleWorkloadRequest, HandleWorkloadResponse};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

pub async fn handle_workload(
    request: Request<HandleWorkloadRequest>,
) -> Result<Response<HandleWorkloadResponse>, Status> {
    // Implement the logic to handle workload requests from ActionController here.
    // For now, we will just return an unimplemented status.
    match crate::runtime::podman::create_nginx_container("test-nginx-container").await {
        Ok(container_id) => {
            println!("Created container with ID: {}", container_id);
            let response = HandleWorkloadResponse {
                status: true,
                desc: format!("Container created"),
            };
            Ok(Response::new(response))
        }
        Err(e) => {
            println!("Failed to create container: {:?}", e);
            Err(Status::unimplemented(
                "handle_workload is not implemented yet",
            ))
        }
    }
}
