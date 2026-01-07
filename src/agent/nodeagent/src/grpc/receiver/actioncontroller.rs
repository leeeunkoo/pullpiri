/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::nodeagent::fromactioncontroller::{HandleWorkloadRequest, HandleWorkloadResponse};
use tonic::{Request, Response, Status};

pub async fn handle_workload(
    request: Request<HandleWorkloadRequest>,
) -> Result<Response<HandleWorkloadResponse>, Status> {
    // Implement the logic to handle workload requests from ActionController here.
    // For now, we will just return an unimplemented status.
    // TODO - Currently, just create a test nginx container for development.
    //        Need to implement actual workload handling logic.
    let req = request.into_inner();
    match crate::runtime::podman::handle_workload(req.workload_command, &req.model_name).await {
        Ok(_) => {
            println!("Workload handle {} successfully", req.workload_command.to_string());
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
