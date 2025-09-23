use common::nodeagent::{
    node_agent_connection_client::NodeAgentConnectionClient, HandleYamlRequest, HandleYamlResponse,
};
use tonic::{Request, Response, Status};

// Send to a specific node using its IP address
pub async fn send_to_node(
    action: HandleYamlRequest,
    node_ip: String,
) -> Result<Response<HandleYamlResponse>, Status> {
    let addr = format!("http://{}:47004", node_ip);

    println!("Attempting to connect to NodeAgent at: {}", addr);

    // Attempting to connect with a timeout
    let client_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        NodeAgentConnectionClient::connect(addr.clone()),
    )
    .await;

    match client_result {
        Ok(Ok(mut client)) => {
            println!("Successfully connected to NodeAgent, sending request...");
            match tokio::time::timeout(
                std::time::Duration::from_secs(10),
                client.handle_yaml(Request::new(action)),
            )
            .await
            {
                Ok(result) => match result {
                    Ok(response) => {
                        println!("Request to NodeAgent successful");
                        Ok(response)
                    }
                    Err(e) => {
                        eprintln!("Error calling NodeAgent handle_yaml: {}", e);
                        Err(Status::internal(format!(
                            "Error calling NodeAgent handle_yaml: {}",
                            e
                        )))
                    }
                },
                Err(_) => {
                    eprintln!("Timeout while waiting for NodeAgent to respond");
                    Err(Status::deadline_exceeded(
                        "Timeout while waiting for NodeAgent to respond",
                    ))
                }
            }
        }
        Ok(Err(e)) => {
            eprintln!("Error connecting to NodeAgent at {}: {}", addr, e);
            eprintln!("Connection error details: {:?}", e);
            Err(Status::unavailable(format!(
                "Failed to connect to NodeAgent at {}: {}",
                addr, e
            )))
        }
        Err(_) => {
            eprintln!("Timeout while connecting to NodeAgent at {}", addr);
            Err(Status::deadline_exceeded(format!(
                "Timeout while connecting to NodeAgent at {}",
                addr
            )))
        }
    }
}

pub async fn send(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    // Use the node lookup module to get the node IP
    let node_ip = crate::node::get_node_ip().await;

    // Send to the node
    send_to_node(action, node_ip).await
}

// etcd에서 게스트 노드 정보를 가져오도록 수정
pub async fn send_guest(
    action: HandleYamlRequest,
) -> Result<Vec<Response<HandleYamlResponse>>, Status> {
    // etcd에서 게스트 노드 정보들을 가져오기
    let guest_nodes = crate::node::node_lookup::find_guest_nodes().await;

    if guest_nodes.is_empty() {
        return Err(Status::not_found("No guest nodes found in etcd"));
    }

    let mut responses = Vec::new();

    for guest_node in guest_nodes {
        println!(
            "Sending to guest node: {} ({})",
            guest_node.node_id, guest_node.ip_address
        );
        match send_to_node(action.clone(), guest_node.ip_address.clone()).await {
            Ok(response) => {
                println!("Successfully sent to guest node: {}", guest_node.node_id);
                responses.push(response);
            }
            Err(e) => {
                println!(
                    "Failed to send to guest node {}: {:?}",
                    guest_node.node_id, e
                );
                // 오류는 기록하지만 계속 다른 노드에 전송 시도
            }
        }
    }

    if responses.is_empty() {
        Err(Status::unavailable("Failed to send to any guest nodes"))
    } else {
        Ok(responses)
    }
}
