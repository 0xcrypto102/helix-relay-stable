use crate::{api::*, ValidatorPreferences};
use clap::Parser;
use ethereum_consensus::ssz::prelude::Node;
use helix_utils::request_encoding::Encoding;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::File};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RelayConfig {
    pub postgres: PostgresConfig,
    pub redis: RedisConfig,
    #[serde(default)]
    pub broadcasters: Vec<BroadcasterConfig>,
    pub simulator: SimulatorConfig,
    #[serde(default)]
    pub beacon_clients: Vec<BeaconClientConfig>,
    #[serde(default)]
    pub relays: Vec<RelayGossipConfig>,
    #[serde(default)]
    pub network_config: NetworkConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub validator_preferences: ValidatorPreferences,
    #[serde(default)]
    pub router_config: RouterConfig,
    #[serde(default = "default_duration")]
    pub target_get_payload_propagation_duration_ms: u64,
}

impl RelayConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let start_config = StartConfig::parse();
        let file = File::open(start_config.config)?;
        let config: RelayConfig = serde_yaml::from_reader(file)?;
        Ok(config)
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PostgresConfig {
    pub hostname: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub db_name: String,
    pub user: String,
    pub password: String,
    #[serde(default)]
    pub region: i16,
    #[serde(default)]
    pub region_name: String,
}

fn default_port() -> u16 {
    5432
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BroadcasterConfig {
    Fiber(FiberConfig),
    BeaconClient(BeaconClientConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FiberConfig {
    pub url: String,
    pub api_key: String,
    pub encoding: Encoding,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SimulatorConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BeaconClientConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RelayGossipConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub enum NetworkConfig {
    #[default]
    Mainnet,
    Goerli,
    Sepolia,
    Holesky,
    Custom {
        dir_path: String,
        genesis_validator_root: Node,
        genesis_time: u64,
    },
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub enum LoggingConfig {
    #[default]
    Console,
    File {
        dir_path: String,
        file_name: String,
    },
}

#[derive(Parser, Debug, Clone, Default, Serialize, Deserialize)]
#[clap(name = "basic")]
pub struct StartConfig {
    #[clap(long, default_value = "config.yml")]
    pub config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterConfig {
    pub enabled_routes: Vec<RouteInfo>,
}

impl RouterConfig {
    // Function to resolve condensed variants and replace them with real routes
    pub fn resolve_condensed_routes(&mut self) {
        if self.enabled_routes.is_empty() {
            // If no routes are enabled, enable all real routes
            self.extend([
                Route::BuilderApi,
                Route::ProposerApi,
                Route::DataApi,
                Route::ConstraintsApi,
            ]);
        } else if self.contains(Route::All) {
            // If All is present, replace it with all real routes
            self.remove(&Route::All);
            self.extend([
                Route::BuilderApi,
                Route::ProposerApi,
                Route::DataApi,
                Route::ConstraintsApi,
            ]);
        }

        // Replace BuilderApi, ProposerApi, DataApi, ConstraintsApi with their real routes
        self.replace_condensed_with_real(
            Route::BuilderApi,
            &[
                Route::GetValidators,
                Route::SubmitBlock,
                Route::SubmitBlockWithProofs,
                Route::SubmitBlockOptimistic,
                Route::SubmitHeader,
                Route::GetTopBid,
                Route::GetBuilderConstraints,
                Route::GetBuilderConstraintsStream,
                Route::GetBuilderDelegations,
                Route::GetBuilderValidatorDelegations,

            ],
        );

        self.replace_condensed_with_real(
            Route::ProposerApi,
            &[
                Route::Status,
                Route::RegisterValidators,
                Route::GetHeader,
                Route::GetHeaderWithProofs,
                Route::GetPayload
            ],
        );

        self.replace_condensed_with_real(
            Route::DataApi,
            &[
                Route::ProposerPayloadDelivered,
                Route::BuilderBidsReceived,
                Route::ValidatorRegistration,
            ],
        );

        self.replace_condensed_with_real(
            Route::ConstraintsApi,
            &[
                Route::SubmitBuilderConstraints,
                Route::DelegateSubmissionRights,
                Route::RevokeSubmissionRights,
            ],
        );
    }

    fn contains(&self, route: Route) -> bool {
        self.enabled_routes.iter().map(|x| x.route).collect::<HashSet<_>>().contains(&route)
    }

    fn remove(&mut self, route: &Route) {
        self.enabled_routes.retain(|x| x.route != *route);
    }

    fn extend(&mut self, routes: impl IntoIterator<Item = Route>) {
        for route in routes {
            if !self.contains(route) {
                self.enabled_routes.push(RouteInfo { route, rate_limit: None });
            }
        }
    }

    fn replace_condensed_with_real(&mut self, special_variant: Route, real_routes: &[Route]) {
        if self.contains(special_variant) {
            self.remove(&special_variant);
            self.extend(real_routes.iter().cloned());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub route: Route,
    pub rate_limit: Option<RateLimitInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    // The duration of the rate limit in milliseconds
    pub limit_duration_ms: u64,
    // The maximum number of requests allowed within the duration
    pub max_requests: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Route {
    All,
    BuilderApi,
    ProposerApi,
    DataApi,
    ConstraintsApi,
    GetValidators,
    SubmitBlock,
    SubmitBlockOptimistic,
    SubmitHeader,
    GetTopBid,
    Status,
    RegisterValidators,
    GetHeader,
    GetPayload,
    ProposerPayloadDelivered,
    BuilderBidsReceived,
    ValidatorRegistration,

    // Constraints API: Builder <https://docs.boltprotocol.xyz/api/builder>
    /// Reference: <https://docs.boltprotocol.xyz/api/builder#constraints>
    SubmitBuilderConstraints,
    /// Reference: <https://docs.boltprotocol.xyz/api/builder#delegate>
    DelegateSubmissionRights,
    /// Reference: <https://docs.boltprotocol.xyz/api/builder#revoke>
    RevokeSubmissionRights,
    /// Reference: <https://docs.boltprotocol.xyz/api/builder#get_header_with_proofs>
    GetHeaderWithProofs,

    // Constraints API: Relay <https://docs.boltprotocol.xyz/api/relay>
    /// Reference: <https://docs.boltprotocol.xyz/api/relay#constraints>
    GetBuilderConstraints,
    /// Reference: <https://docs.boltprotocol.xyz/api/relay#constraints_stream>
    GetBuilderConstraintsStream,
    GetBuilderDelegations,
    GetBuilderValidatorDelegations,
    /// Reference: <https://docs.boltprotocol.xyz/api/relay#blocks_with_proofs>
    SubmitBlockWithProofs,
}

impl Route {
    pub fn path(&self) -> String {
        match self {
            Route::GetValidators => format!("{PATH_BUILDER_API}{PATH_GET_VALIDATORS}"),
            Route::SubmitBlock => format!("{PATH_BUILDER_API}{PATH_SUBMIT_BLOCK}"),
            Route::SubmitBlockWithProofs => format!("{PATH_BUILDER_API}{PATH_BUILDER_BLOCKS_WITH_PROOFS}"),
            Route::SubmitBlockOptimistic => format!("{PATH_BUILDER_API}{PATH_SUBMIT_BLOCK_OPTIMISTIC_V2}"),
            Route::SubmitHeader => format!("{PATH_BUILDER_API}{PATH_SUBMIT_HEADER}"),
            Route::GetTopBid => format!("{PATH_BUILDER_API}{PATH_GET_TOP_BID}"),
            Route::Status => format!("{PATH_PROPOSER_API}{PATH_STATUS}"),
            Route::RegisterValidators => format!("{PATH_PROPOSER_API}{PATH_REGISTER_VALIDATORS}"),
            Route::GetHeader => format!("{PATH_PROPOSER_API}{PATH_GET_HEADER}"),
            Route::GetHeaderWithProofs => format!("{PATH_PROPOSER_API}{PATH_GET_HEADER_WITH_PROOFS}"),
            Route::GetPayload => format!("{PATH_PROPOSER_API}{PATH_GET_PAYLOAD}"),
            Route::ProposerPayloadDelivered => format!("{PATH_DATA_API}{PATH_PROPOSER_PAYLOAD_DELIVERED}"),
            Route::BuilderBidsReceived => format!("{PATH_DATA_API}{PATH_BUILDER_BIDS_RECEIVED}"),
            Route::ValidatorRegistration => format!("{PATH_DATA_API}{PATH_VALIDATOR_REGISTRATION}"),
            Route::SubmitBuilderConstraints => format!("{PATH_CONSTRAINTS_API}{PATH_SUBMIT_BUILDER_CONSTRAINTS}"),
            Route::DelegateSubmissionRights => format!("{PATH_CONSTRAINTS_API}{PATH_DELEGATE_SUBMISSION_RIGHTS}"),
            Route::RevokeSubmissionRights => format!("{PATH_CONSTRAINTS_API}{PATH_REVOKE_SUBMISSION_RIGHTS}"),
            Route::GetBuilderConstraints => format!("{PATH_BUILDER_API}{PATH_BUILDER_CONSTRAINTS}"),
            Route::GetBuilderConstraintsStream => format!("{PATH_BUILDER_API}{PATH_BUILDER_CONSTRAINTS_STREAM}"),
            Route::GetBuilderDelegations => format!("{PATH_BUILDER_API}{PATH_BUILDER_DELEGATIONS}"),
            Route::GetBuilderValidatorDelegations => format!("{PATH_BUILDER_API}{PATH_BUILDER_VALIDATOR_DELEGATIONS}"),
            Route::All => panic!("All is not a real route"),
            Route::BuilderApi => panic!("BuilderApi is not a real route"),
            Route::ProposerApi => panic!("ProposerApi is not a real route"),
            Route::DataApi => panic!("DataApi is not a real route"),
            Route::ConstraintsApi => panic!("ConstraintsApi is not a real route"),
        }
    }
}

fn default_duration() -> u64 {
    1000
}

#[cfg(test)]
#[test]
fn test_config() {
    use crate::{Filtering, ValidatorPreferences};

    let mut config = RelayConfig::default();
    config.redis.url = "redis://localhost:6379".to_string();
    config.simulator.url = "http://localhost:8080".to_string();
    config.beacon_clients.push(BeaconClientConfig { url: "http://localhost:8080".to_string() });
    config.broadcasters.push(BroadcasterConfig::BeaconClient(BeaconClientConfig {
        url: "http://localhost:8080".to_string(),
    }));
    config.network_config = NetworkConfig::Custom {
        dir_path: "test".to_string(),
        genesis_validator_root: Default::default(),
        genesis_time: 1,
    };
    config.logging =
        LoggingConfig::File { dir_path: "hello".to_string(), file_name: "test".to_string() };
    config.validator_preferences = ValidatorPreferences {
        filtering: Filtering::Regional,
        trusted_builders: None,
        header_delay: true,
    };
    config.router_config = RouterConfig {
        enabled_routes: vec![
            RouteInfo { route: Route::GetValidators, rate_limit: None },
            RouteInfo { route: Route::SubmitBlock, rate_limit: None },
            RouteInfo { route: Route::SubmitBlockOptimistic, rate_limit: None },
            RouteInfo { route: Route::ValidatorRegistration, rate_limit: None },
            RouteInfo {
                route: Route::GetHeader,
                rate_limit: Some(RateLimitInfo { limit_duration_ms: 12, max_requests: 3 }),
            },
            RouteInfo { route: Route::GetPayload, rate_limit: None },
            RouteInfo { route: Route::ProposerPayloadDelivered, rate_limit: None },
            RouteInfo { route: Route::RegisterValidators, rate_limit: None },
            RouteInfo { route: Route::Status, rate_limit: None },
        ]
        .to_vec(),
    };
    println!("{}", serde_yaml::to_string(&config).unwrap());
}
