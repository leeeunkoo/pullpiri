use if_addrs::{get_if_addrs, Interface};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;
use thiserror::Error;

// Global config instance
static NODEAGENT_CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MetricsConfig {
    pub collection_interval: u64,
    pub batch_size: u32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SystemConfig {
    pub hostname: String,
    pub platform: String,
    pub architecture: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NodeAgentConfig {
    #[serde(default = "default_node_name")]
    pub node_name: String,
    #[serde(default = "default_node_type")]
    pub node_type: String,
    #[serde(default = "default_node_role")]
    pub node_role: String,
    pub master_ip: String,
    #[serde(default)]
    pub node_ip: String,
    pub grpc_port: u16,
    pub log_level: String,
    pub metrics: MetricsConfig,
    pub system: SystemConfig,
    #[serde(default = "default_yaml_storage")]
    pub yaml_storage: String,
}

fn default_node_name() -> String {
    match hostname::get() {
        Ok(hostname) => hostname.to_string_lossy().to_string(),
        Err(_) => "unknown".to_string(),
    }
}

fn default_node_type() -> String {
    "cloud".to_string()
}

fn default_node_role() -> String {
    "nodeagent".to_string()
}

fn default_yaml_storage() -> String {
    "/etc/piccolo/yaml".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub nodeagent: NodeAgentConfig,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_host_ip(&self) -> String {
        // If node_ip is explicitly set in config, use it
        if !self.nodeagent.node_ip.is_empty() {
            return self.nodeagent.node_ip.clone();
        }

        // Otherwise try to get the first non-loopback IPv4 address
        if let Ok(interfaces) = get_network_interfaces() {
            for iface in interfaces {
                if let std::net::IpAddr::V4(ipv4) = iface.addr.ip() {
                    if !ipv4.is_loopback() {
                        return ipv4.to_string();
                    }
                }
            }
        }

        // Fallback to master_ip if we couldn't determine the host IP
        self.nodeagent.master_ip.clone()
    }

    pub fn get_hostname(&self) -> String {
        self.nodeagent.system.hostname.clone()
    }

    pub fn get_node_name(&self) -> String {
        self.nodeagent.node_name.clone()
    }

    pub fn get_yaml_storage(&self) -> String {
        self.nodeagent.yaml_storage.clone()
    }

    // Get or initialize the global config
    pub fn get() -> &'static Config {
        NODEAGENT_CONFIG.get().unwrap_or_else(|| {
            let default_config = Config::default();
            NODEAGENT_CONFIG.set(default_config.clone()).unwrap_or(());
            NODEAGENT_CONFIG.get().unwrap()
        })
    }

    // Set the global config
    pub fn set_global(config: Config) {
        let _ = NODEAGENT_CONFIG.set(config);
    }
}

// Helper function to get network interfaces
fn get_network_interfaces() -> Result<Vec<Interface>, std::io::Error> {
    get_if_addrs()
}
