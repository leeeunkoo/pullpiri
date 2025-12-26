use common::nodeagent::fromactioncontroller::{
    connect_server, handle_workload_connection_client::HandleWorkloadConnectionClient,
    HandleWorkloadRequest, HandleWorkloadResponse, WorkloadCommand,
};
use tonic::{Request, Status};

pub async fn send_workload_handle_request(
    addr: &str,
    request: HandleWorkloadRequest,
) -> Result<HandleWorkloadResponse, Status> {
    let mut client = HandleWorkloadConnectionClient::connect(connect_server(&addr))
        .await
        .unwrap();

    let response = client
        .handle_workload(Request::new(request))
        .await?
        .into_inner();
    Ok(response)
}
