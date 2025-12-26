/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

use hyper::{Body, Client, Method, Request, Uri};
use hyperlocal::{UnixConnector, Uri as UnixUri};
use serde_json::json;

pub async fn get(path: &str) -> Result<hyper::body::Bytes, hyper::Error> {
    let connector = UnixConnector;
    let client = Client::builder().build::<_, Body>(connector);

    // Modify this if you want to run without root authorization
    // or if you have a different socket path.
    // For example, if you run Podman as root, you might use:
    // let socket = "/var/run/podman/podman.sock";
    // Or if you run it as a user, you might use:
    // let socket = "/run/user/1000/podman/podman.sock
    let socket = "/var/run/podman/podman.sock";
    // let socket = "/var/run/podman/podman.sock";
    let uri: Uri = UnixUri::new(socket, path).into();

    let res = client.get(uri).await?;
    hyper::body::to_bytes(res).await
}

pub async fn post(path: &str, body: Body) -> Result<hyper::body::Bytes, hyper::Error> {
    let connector = UnixConnector;
    let client = Client::builder().build::<_, Body>(connector);

    // Modify this if you want to run without root authorization
    // or if you have a different socket path.
    // For example, if you run Podman as root, you might use:
    // let socket = "/var/run/podman/podman.sock";
    // Or if you run it as a user, you might use:
    // let socket = "/run/user/1000/podman/podman.sock
    let socket = "/var/run/podman/podman.sock";
    // let socket = "/var/run/podman/podman.sock";
    // let path = "/v4.0.0/libpod/containers/{name}/start";
    let uri: Uri = UnixUri::new(socket, path).into();

    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .body(body)
        .unwrap();

    let res = client.request(req).await?;
    hyper::body::to_bytes(res).await
}

/// Check if an image exists locally
pub async fn image_exists(image_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let path = "/v4.0.0/libpod/images/json";
    /*match get(&path).await {
        Ok(_) => Ok(true),
        Err(e) => {
            // If we get an error, check if it's a 404 (image not found)
            // For simplicity, we'll treat any error as "image doesn't exist"
            Ok(false)
        }
    }*/
    let result = get(path).await?;
    let images: Vec<serde_json::Value> = serde_json::from_slice(&result)?;
    for image in images {
        if let Some(repo_tags) = image["RepoTags"].as_array() {
            for tag in repo_tags {
                if tag.as_str() == Some(image_name) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Pull an image from a registry
pub async fn pull_image(image_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("/v4.0.0/libpod/images/pull?reference={}", image_name);
    post(&path, Body::empty()).await?;
    Ok(())
}

/// Create and start an nginx container
/// Returns the container ID on success
pub async fn create_nginx_container(
    container_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let image_name = "docker.io/library/nginx:latest";

    // Step 0: Check if image exists, pull if not
    if !image_exists(image_name).await? {
        println!("Image {} not found locally, pulling...", image_name);
        pull_image(image_name).await?;
        println!("Image {} pulled successfully", image_name);
    }

    // Step 1: Create the container
    let create_body = json!({
        "Image": "docker.io/library/nginx:latest",
        "Name": container_name,
        "HostConfig": {
            "PortBindings": {
                "80/tcp": [
                    {
                        "HostPort": "8080"
                    }
                ]
            }
        }
    });

    let create_path = "/v4.0.0/libpod/containers/create";
    let create_response = post(create_path, Body::from(create_body.to_string())).await?;

    let create_result: serde_json::Value = serde_json::from_slice(&create_response)?;
    let container_id = create_result["Id"]
        .as_str()
        .ok_or("Failed to get container ID")?
        .to_string();

    // Step 2: Start the container
    let start_path = format!("/v4.0.0/libpod/containers/{}/start", container_id);
    post(&start_path, Body::empty()).await?;

    Ok(container_id)
}

//Unit tets cases
#[cfg(test)]
mod tests {
    use super::get;
    use hyper::body::Bytes;
    use hyper::Error;
    use tokio;

    #[tokio::test]
    async fn test_get_with_valid_path() {
        let result: Result<Bytes, Error> = get("/v1.0/version").await;
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
