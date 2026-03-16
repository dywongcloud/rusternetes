pub mod annotate;
pub mod api_resources;
pub mod api_versions;
pub mod apply;
pub mod auth;
pub mod cluster_info;
pub mod config;
pub mod cp;
pub mod create;
pub mod delete;
pub mod describe;
pub mod diff;
pub mod edit;
pub mod exec;
pub mod explain;
pub mod get;
pub mod label;
pub mod logs;
pub mod patch;
pub mod port_forward;
pub mod rollout;
pub mod scale;
pub mod top;
pub mod version;
pub mod wait;

#[cfg(test)]
mod apply_test;
#[cfg(test)]
mod create_test;
#[cfg(test)]
mod get_test;
