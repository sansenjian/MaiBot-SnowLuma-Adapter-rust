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
| `CORE_URL` | `http://127.0.0.1:8000` | Core 的 socket.io 服务端地址 |
| `CORE_TOKEN` | 空 | Core `enable_token` 时的 Authorization token |
| `PLATFORM` | `qq` | 连接 Core 的 platform 头 |
| `DRIVER_ID` | `legacy.send.qq` | 去重键前缀（对齐 Core manager.py 约定） |

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
- 入站：OneBot message 事件 → MessageBase（纯同步子集，见下）
- 去重：`{driver_id}:{message_id}`，TTL 300s / 上限 10000
- 上行：socket.io client（platform/Authorization 头、单事件 `message`、重连退避）
- 出站：MessageBase/MessageDict 统一 → OneBot send_* 动作（16 类段映射 + forward 打包）

## 已知差距（v1，行为对齐待办）

1. at 段昵称解析（需 NapCat get_group_member_info/get_stranger_info 查询通道）
2. reply 段被引消息预览（需 get_msg 查询）
3. 图片/语音下载转 base64（当前直接用 url，MessageBase 惯例可接受）
4. forward 段 get_forward_msg 查询（无内联节点时降级为 `[forward]` 文本）
5. QQ_FACE 表情表（当前统一 `[表情]`）
6. echo 回执仅记日志，未与发送方关联（出站追踪）
7. 非 message 事件（notice/request/meta_event）直接忽略
8. rust-socketio API 版本差异可能导致编译需微调（`connect().await`、Payload 变体）

## 验收对照

行为对齐的依据与全量规则：`docs/event-schema.md`（§5 wire 协议、§7 载荷形态、§8 转换规则）。
