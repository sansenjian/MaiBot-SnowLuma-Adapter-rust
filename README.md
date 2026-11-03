# maibot-rust

MaiBot（麦麦）的 Rust 消息网关，绞杀者模式逐步替换官方 Python 适配器进程。
对协议端说 OneBot v11，对 MaiBot Core 说 plugin RPC（或 classic socket.io）。

[![CI](https://github.com/sansenjian/MaiBot-SnowLuma-Adapter-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/sansenjian/MaiBot-SnowLuma-Adapter-rust/actions/workflows/ci.yml)

```
SnowLuma / NapCat (OneBot v11 WS) ⇄ maibot-rust ⇄ MaiBot Core
                        协议端画像              plugin RPC / socket.io
```

> 背景：官方 Napcat-Adapter 已停止更新，后继为 SnowLuma-Adapter（本仓库的转换规则对齐它）。

## 特性

- **双上行模式**：`plugin`（Core plugin_runtime 的 Runner，msgpack-RPC over TCP，MessageDict 载荷——当前生态主线）/ `socketio`（Core classic MessageServer，MessageBase 载荷）
- **协议端画像**：`snowluma`（默认，通用 `send_msg`+`message_type`、file 拆 `upload_*_file` 保序、合并转发键 `messages`）/ `napcat`（`send_group/private_msg`、file 内联、键 `message`）；token 双通道（Bearer 头 + URL `access_token`）
- **入站对齐 SnowLuma-Adapter**：at 昵称解析（`get_group_member_info`/`get_stranger_info`）、reply 被引预览（`get_msg`）、forward 查询（`get_forward_msg`）、图片下载转 base64（subType 判 emoji）、语音 `get_record` 取 base64、QQ 表情表（700+ 条，emoji 近似 / 描述 / Unicode 码点）
- **去重**：socketio 模式本地按 `{driver_id}:{message_id}`（TTL 300s / 上限 1 万）；plugin 模式交给 Host 用 dedupe_key
- **出站完整协议**：串行 await 平台回执、`external_message_id` 回填、`message_id_echo` adapter_callbacks——与官方 adapter 行为一致
- **健壮性**：双端指数退避重连、WS 协议层 Ping/Pong、收帧分派与业务转换隔离（无自死锁）、未注册 RPC 方法回 `E_METHOD_NOT_ALLOWED`

## 快速开始

```bash
git clone https://github.com/sansenjian/MaiBot-SnowLuma-Adapter-rust.git
cd maibot-rust
cargo build --release
```

### 配置（环境变量）

| 变量 | 默认 | 说明 |
|---|---|---|
| `ONEBOT_WS_URL` | `ws://127.0.0.1:3001` | 协议端正向 WS 地址（SnowLuma 默认 :3001） |
| `NAPCAT_TOKEN` | 空 | 鉴权 token（Bearer 头；snowluma 画像下同时附加 `?access_token=`） |
| `UPSTREAM_MODE` | `plugin` | `plugin` / `socketio` |
| `PROTOCOL_END` | `snowluma` | 协议端画像 `snowluma` / `napcat` |
| `PLUGIN_HOST_ADDRESS` | `127.0.0.1:0` | plugin 模式：Core Host IPC 地址（TCP） |
| `PLUGIN_SESSION_TOKEN` | 空 | plugin 模式：握手会话令牌（Core 启动 Runner 时下发） |
| `PLUGIN_ID` | `maibot_gateway_rust` | 注册的 plugin_id |
| `GATEWAY_NAME` | `napcat_gateway` | MessageGateway 组件名 |
| `PLATFORM` | `qq` | platform 声明 |
| `SELF_ID` | 空 | 机器人 QQ 号（写入 additional_config.self_id） |
| `CORE_URL` / `CORE_TOKEN` | `http://127.0.0.1:8000` / 空 | socketio 模式专用 |
| `DRIVER_ID` | `legacy.send.qq` | 去重键前缀 |

### 运行

```bash
cargo run --release        # 生产
RUST_LOG=maibot_gateway=debug cargo run   # 调试
```

## 测试

```bash
cargo test                     # 29 个单元测试
pip install msgpack websockets
python3 e2e/mock_test.py       # 22 项端到端断言（mock SnowLuma + mock Core Host）
```

CI（GitHub Actions）在每次推送时自动跑以上全部。

## 资源占用（实测）

release 二进制 7.2 MB；mock 全链路流量下 RSS 峰值约 12 MB，满负荷预估 15-16 MB（去重表 + 队列有硬上限）。无泄漏迹象（短窗口观察）。

## 已知差距

- 文本→at 转换（对齐 `enable_text_at_conversion`，需群成员列表查询）
- 非 message 事件（notice/request/meta_event）当前忽略

## 协议文档

`docs/event-schema.md`：OneBot 事件契约、MessageBase/MessageDict 双载荷、plugin RPC（Envelope/握手/方法表）、收发转换规则全表。

## License

GPL-3.0（与 MaiBot 生态一致）
