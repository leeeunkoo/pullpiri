/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

use super::{get, post};
use hyper::Body;
use serde_json::json;

static MODEL_DIR: &str = "/etc/piccolo/yaml";

pub async fn start(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = format!("{}/{}.yaml", MODEL_DIR, model_name);
    println!("model_path: {}", model_path);
    let text = std::fs::read_to_string(&model_path)?;
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(&text)?;

    // Get pod name for container naming
    let pod_name = pod.get_name();

    // Extract the spec to access containers
    let pod_json = serde_json::to_value(&pod)?;
    let spec = pod_json["spec"].clone();

    // Get hostNetwork setting from Pod spec
    let host_network = spec["hostNetwork"].as_bool().unwrap_or(false);

    // Process containers
    if let Some(containers) = spec["containers"].as_array() {
        for (_, container) in containers.iter().enumerate() {
            let image = container["image"]
                .as_str()
                .ok_or("Container image field not found")?;
            let container_name = container["name"]
                .as_str()
                .ok_or("Container name field not found")?;

            // Check if image exists, pull if not
            if !image_exists(image).await? {
                println!("Image {} not found locally, pulling...", image);
                pull_image(image).await?;
                println!("Image {} pulled successfully", image);
            }

            // Build container creation request
            let mut create_body = json!({
                "Image": image,
                "Name": format!("{}_{}", pod_name, container_name),
            });

            // Add hostNetwork setting
            if host_network {
                if create_body.get("HostConfig").is_none() {
                    create_body["HostConfig"] = json!({});
                }
                create_body["HostConfig"]["NetworkMode"] = json!("host");
            }

            // Add port bindings if ports are specified
            if let Some(ports) = container["ports"].as_array() {
                let mut port_bindings = serde_json::Map::new();
                for port in ports {
                    if let Some(container_port) = port["containerPort"].as_i64() {
                        let host_port = port["hostPort"].as_i64().unwrap_or(container_port);
                        let key = format!("{}/tcp", container_port);
                        port_bindings.insert(key, json!([{"HostPort": host_port.to_string()}]));
                    }
                }
                if !port_bindings.is_empty() {
                    create_body["HostConfig"] = json!({
                        "PortBindings": port_bindings
                    });
                }
            }

            // Add environment variables if specified
            if let Some(env) = container["env"].as_array() {
                let env_vars: Vec<String> = env
                    .iter()
                    .filter_map(|e| {
                        let name = e["name"].as_str()?;
                        let value = e["value"].as_str()?;
                        Some(format!("{}={}", name, value))
                    })
                    .collect();
                if !env_vars.is_empty() {
                    create_body["Env"] = json!(env_vars);
                }
            }

            // Add command if specified
            if let Some(command) = container["command"].as_array() {
                let cmd: Vec<String> = command
                    .iter()
                    .filter_map(|c| c.as_str().map(|s| s.to_string()))
                    .collect();
                if !cmd.is_empty() {
                    create_body["Cmd"] = json!(cmd);
                }
            }

            // Add volume mounts if specified
            if let Some(volume_mounts) = container["volumeMounts"].as_array() {
                if let Some(volumes) = spec["volumes"].as_array() {
                    let mut binds = Vec::new();
                    for mount in volume_mounts {
                        let mount_name = mount["name"].as_str().unwrap_or("");
                        let mount_path = mount["mountPath"].as_str().unwrap_or("");

                        // Find corresponding volume
                        for volume in volumes {
                            if volume["name"].as_str() == Some(mount_name) {
                                if let Some(host_path) = volume["hostPath"]["path"].as_str() {
                                    binds.push(format!("{}:{}", host_path, mount_path));
                                }
                                break;
                            }
                        }
                    }
                    if !binds.is_empty() {
                        if create_body.get("HostConfig").is_none() {
                            create_body["HostConfig"] = json!({});
                        }
                        create_body["HostConfig"]["Binds"] = json!(binds);
                    }
                }
            }

            // Create the container
            println!("Creating container from image: {}", image);
            let create_path = "/v4.0.0/libpod/containers/create";
            let create_response = post(create_path, Body::from(create_body.to_string())).await?;

            let create_result: serde_json::Value = serde_json::from_slice(&create_response)?;
            let container_id = create_result["Id"]
                .as_str()
                .ok_or("Failed to get container ID")?;

            // Start the container
            println!("Starting container: {}", container_id);
            let start_path = format!("/v4.0.0/libpod/containers/{}/start", container_id);
            post(&start_path, Body::empty()).await?;

            println!("Container {} started successfully", container_id);
        }
    }

    Ok(())
}

pub async fn stop(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = format!("{}/{}.yaml", MODEL_DIR, model_name);
    println!("model_path: {}", model_path);
    let text = std::fs::read_to_string(&model_path)?;
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(&text)?;

    // Get pod name for container naming
    let pod_name = pod.get_name();

    // Extract the spec to access containers
    let pod_json = serde_json::to_value(&pod)?;
    let spec = pod_json["spec"].clone();

    // Process containers
    if let Some(containers) = spec["containers"].as_array() {
        for (_, container) in containers.iter().enumerate() {
            let container_name = container["name"]
                .as_str()
                .ok_or("Container name field not found")?;

            let full_container_name = format!("{}_{}", pod_name, container_name);

            // Stop the container
            println!("Stopping container: {}", full_container_name);
            let stop_path = format!("/v4.0.0/libpod/containers/{}/stop", full_container_name);
            match post(&stop_path, Body::empty()).await {
                Ok(_) => println!("Container {} stopped successfully", full_container_name),
                Err(e) => println!(
                    "Warning: Failed to stop container {}: {}",
                    full_container_name, e
                ),
            }

            // Remove the container
            println!("Removing container: {}", full_container_name);
            let remove_path = format!(
                "/v4.0.0/libpod/containers/{}?force=true",
                full_container_name
            );
            match super::delete(&remove_path).await {
                Ok(_) => println!("Container {} removed successfully", full_container_name),
                Err(e) => println!(
                    "Warning: Failed to remove container {}: {}",
                    full_container_name, e
                ),
            }
        }
    }

    Ok(())
}

pub async fn restart(model_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let model_path = format!("{}/{}.yaml", MODEL_DIR, model_name);
    println!("model_path: {}", model_path);
    let text = std::fs::read_to_string(&model_path)?;
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(&text)?;

    // Get pod name for container naming
    let pod_name = pod.get_name();

    // Extract the spec to access containers
    let pod_json = serde_json::to_value(&pod)?;
    let spec = pod_json["spec"].clone();

    // Process containers
    if let Some(containers) = spec["containers"].as_array() {
        for (_, container) in containers.iter().enumerate() {
            let container_name = container["name"]
                .as_str()
                .ok_or("Container name field not found")?;

            let full_container_name = format!("{}_{}", pod_name, container_name);

            // Use Podman's restart API endpoint
            println!("Restarting container: {}", full_container_name);
            let restart_path = format!("/v4.0.0/libpod/containers/{}/restart", full_container_name);
            match post(&restart_path, Body::empty()).await {
                Ok(_) => println!("Container {} restarted successfully", full_container_name),
                Err(e) => {
                    println!(
                        "Warning: Failed to restart container {}: {}",
                        full_container_name, e
                    );
                    println!("Attempting full stop/start cycle...");
                    // Fallback: if restart fails, try stop and start
                    stop(model_name).await?;
                    start(model_name).await?;
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

/// Check if an image exists locally
pub async fn image_exists(image_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let path = "/v4.0.0/libpod/images/json";

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
