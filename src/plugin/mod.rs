//! 方案 B：Core plugin_runtime Runner 端实现（msgpack-RPC over TCP）。
//! 协议文档见 docs/event-schema.md §9；method 名常量见 envelope.rs。

pub mod client;
pub mod envelope;
pub mod gateway;
