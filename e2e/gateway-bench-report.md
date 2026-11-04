FLOOD DONE
=== RUST GATEWAY BENCH v3 (nodelay fix) ===
messages: 300, received: 300, missing: 0
latency ms: p50=0.09 p95=0.22 p99=0.3 max=0.93 mean=0.11
throughput: 58.1 msg/s (wall 5.16s)
=== PYTHON ADAPTER CODEC (conversion only) ===
py codec baseline unavailable: AttributeError("module 'sla.codecs.inbound.message_codec' has no attribute 'NapCatMessageCodec'")

## Python 对照组（补充实测 2026-11-05）

SnowLuma-Adapter `QQInboundCodec.build_message_dict` 纯文本消息：0.006 ms/msg（仅转换，不含 WS/事件循环/IPC）。
注意口径差异：这是转换函数耗时；Rust 网关 0.09ms p50 是端到端（WS 接收+转换+MessageDict+msgpack+TCP+Host 落库）。
Python 适配器端到端在 asyncio 下的每消息开销（事件循环切换+IPC 序列化）未单独测量，需整机对比。
