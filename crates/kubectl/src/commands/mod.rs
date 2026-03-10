pub mod get;
pub mod create;
pub mod delete;
pub mod apply;
pub mod describe;
pub mod logs;
pub mod exec;
pub mod port_forward;
pub mod cp;
pub mod edit;
pub mod patch;
pub mod scale;
pub mod rollout;
pub mod top;
pub mod label;
pub mod annotate;
pub mod explain;
pub mod wait;
pub mod diff;
pub mod auth;
pub mod api_resources;
pub mod api_versions;
pub mod config;
pub mod cluster_info;
pub mod version;

#[cfg(test)]
mod apply_test;
#[cfg(test)]
mod get_test;
#[cfg(test)]
mod create_test;
