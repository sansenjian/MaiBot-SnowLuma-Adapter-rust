# maibot-gateway

MaiBot 的 Rust 消息网关（绞杀者模式 Phase 1，方案 A）：替换 MaiBot-Napcat-Adapter 独立进程。

```
NapCat(OneBot v11 WS 服务端) ⇄ maibot-gateway ⇄ Socket.IO(maim_message) ⇄ MaiBot Core
```

## 配置（环境变量）

| 变量 | 默认 | 说明 |
|---|---|---|
| `ONEBOT_WS_URL` | `ws://127.0.0.1:3001` | NapCat 正向 WS 地址 |
| `NAPCAT_TOKEN` | 空 | NapCat WS 的 Bearer token（不填不发头） |
| `UPSTREAM_MODE` | `socketio` | `socketio`（方案 A）或 `plugin`（方案 B） |
| `CORE_URL` | `http://127.0.0.1:8000` | socketio 模式：Core 的服务端地址 |
| `CORE_TOKEN` | 空 | socketio 模式：Core Authorization token |
| `PLUGIN_HOST_ADDRESS` | `127.0.0.1:0` | plugin 模式：Host IPC 地址（TCP） |
| `PLUGIN_SESSION_TOKEN` | 空 | plugin 模式：握手会话令牌（Core 启动 Runner 时经环境变量下发，变量名待从 Core 启动代码核对） |
| `PLUGIN_ID` | `maibot_gateway_rust` | plugin 模式：注册的 plugin_id |
| `GATEWAY_NAME` | `napcat_gateway` | plugin 模式：MessageGateway 组件名 |
| `PLATFORM` | `qq` | platform 头 / 网关 platform 声明 |
| `SELF_ID` | 空 | 机器人 QQ 号（plugin 模式写入 additional_config.self_id） |
| `DRIVER_ID` | `legacy.send.qq` | 去重键前缀 |

## 两种上行模式

**socketio（方案 A，默认）**：对 Core classic MessageServer 说话 socket.io client，
载荷 MessageBase，本地按 `{driver_id}:{message_id}` 去重。Core 需 stable 独立
Adapter 部署形态。

**plugin（方案 B）**：作为 Core plugin_runtime 的 Runner 运行，msgpack-RPC over
TCP（4 字节大端帧头 + msgpack Envelope），注册 `MESSAGE_GATEWAY` 组件（duplex /
platform / napcat）。入站 MessageBase → MessageDict → `host.route_message`
（Host 侧用 dedupe_key 去重）；出站 Host 调 `plugin.invoke_message_gateway`，
args.message 直接进现有出站编码管线。⚠️ `host.route_message` /
`plugin.invoke_message_gateway` 两个 method 名为推断值，首次联调核对；
Runner 启动环境变量名同理待核对。

## Core 侧前提

Core 需运行 maim_message 的 MessageServer（ws 模式，路径 `/ws`），即经典独立
Adapter 部署形态。socketio ping 25s/20s 由 python-socketio 服务端处理，客户端无需配置。

## 运行

```bash
cargo run            # 开发
cargo run --release  # 生产（LTO）
```

日志默认 `maibot_gateway=info`，调试用 `RUST_LOG=maibot_gateway=debug`。

## 已实现

- NapCat 正向 WS（Bearer token 头、指数退避重连、出站动作写任务 + echo 计数）
- NapCat API 查询客户端（echo 回执关联，5s 超时）
- 入站：OneBot message 事件 → MessageBase；at 昵称解析（get_group_member_info/get_stranger_info）、reply 被引预览（get_msg）、forward 查询（get_forward_msg，失败降级）
- 去重：`{driver_id}:{message_id}`，TTL 300s / 上限 10000
- 上行：socket.io client（platform/Authorization 头、单事件 `message`、重连退避）
- 出站：MessageBase/MessageDict 统一 → OneBot send_* 动作（16 类段映射 + forward 打包）

## 已知差距（v2，行为对齐待办）

1. 图片/语音下载转 base64（当前 image/voice 段直接用 url，MessageBase 惯例可接受）
2. QQ_FACE 表情表（当前统一 `[表情]`）
3. echo 回执仅记日志，未与发送方关联（出站追踪）
4. 非 message 事件（notice/request/meta_event）直接忽略
5. rust-socketio API 版本差异可能导致编译需微调（`connect().await`、Payload 变体）

## 验收对照

行为对齐的依据与全量规则：`docs/event-schema.md`（§5 wire 协议、§7 载荷形态、§8 转换规则）。
