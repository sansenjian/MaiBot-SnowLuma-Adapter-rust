# MaiBot 事件契约 Schema（P0 交付物 v1）

> 来源：MaiBot main 分支 `src/platform_io/`（types.py / dedupe.py / routing.py /
> route_key_factory.py，2026-11-03 抓取）、maim_message 仓库 master
> `src/maim_message/message_base.py` 逐字段整理。
> 下游 wire 协议（§5）与收发转换规则（§6/§7）已于 2026-11-03 从 maim_message
> master 与 MaiBot-Napcat-Adapter main 提取完毕。

## 0. 部署拓扑（重要修正）

当前 MaiBot（v1.3.x）是**三进程**架构，不是两进程：

```
NapCat ⇄ OneBot v11 WS⇄ MaiBot-Napcat-Adapter（独立 Python 进程）
                  ⇄ maim_message 协议（socketio/WS）⇄ MaiBot Core（platform_io Broker）
```

- 网关与核心之间的协议 = **maim_message**（不是裸 OneBot JSON）。
- Rust 网关 `maibot-gateway` 绞杀的对象 = **Adapter 进程 + Core 内 platform_io 的
  legacy 链路**。网关对 NapCat 说 OneBot v11，对 Core 说 maim_message。
- platform_io 内部另有 RouteKey/RouteTable/Deduper 三层契约（见 §3），
  这些是 Core 内 Broker 的行为，网关在构造去重键/路由键时必须逐语义对齐。

## 1. OneBot v11 入站事件（NapCat → 网关）

信封字段（所有事件共有）：

| 字段 | 类型 | 说明 |
|---|---|---|
| time | f64 | 事件时间戳（秒） |
| self_id | i64? | 机器人 QQ 号 |
| post_type | str | `message` / `notice` / `request` / `meta_event` |

`post_type = message` 时附加字段（未知字段一律容忍——各实现有方言差异）：

| 字段 | 类型 | 说明 |
|---|---|---|
| message_type | str | `group` / `private` |
| sub_type | str | `normal` / `anonymous` / `friend` 等 |
| message_id | i64 | 平台消息 ID（去重键来源） |
| user_id | i64 | 发送者 |
| group_id | i64? | 群号（群消息） |
| raw_message | str | 原始文本（CQ 码） |
| message | array | 消息段数组（text/image/at/face/reply…） |
| sender | obj | `{user_id, nickname, card, role, title}` |

Rust 对应：`src/onebot/event.rs::OneBotEvent`（核心字段 + `#[serde(flatten)]` 收纳其余）。

## 2. maim_message 消息模型（网关 → Core 的上行载荷）

`MessageBase`（对齐 message_base.py）：

```
MessageBase {
  message_info: BaseMessageInfo,
  message_segment: Seg,
  raw_message?: str        # 即将移除，勿依赖
}

BaseMessageInfo {
  platform?: str, message_id?: str, time?: f64,
  group_info?:   { platform?, group_id?, group_name? },
  user_info?:    { platform?, user_id?, user_nickname?, user_cardname? },
  format_info?:  { content_format?: [str], accept_format?: [str] },
  template_info?: { template_items?: {str:str}, template_name?: {str:str}, template_default?: bool=true },
  additional_config?: dict,     # 路由归属字段的兼容载体（见 §3）
  sender_info?:  InfoBase, receiver_info?: InfoBase
}

Seg {
  type: str,                    # text | image | seglist | …
  data: str | [Seg]             # seglist 时 data 为 Seg 数组（可嵌套）
}
```

序列化规则（对齐 to_dict/from_dict，Rust 侧必须遵守）：
- 所有 `Optional` 字段为 None 时**省略该键**（不是置 null）。
- `group_id` 为 None 时整个 `group_info` 为 None。
- seglist 的 data 递归序列化为 `{type, data}` 数组。

## 3. platform_io Broker 契约（Core 侧行为，网关需对齐）

### 3.1 RouteKey（types.py）

- 字段：`platform`（trim + 小写，非空）、`account_id?`、`scope?`（trim，空串→None）。
- `resolution_order()`：从最具体到最宽泛回退：
  - 全键 `(p,a,s)` → `(p,a,None)` → `(p,None,s)` → `(p,None,None)`
  - 只有两个或一个维度时相应缩短，默认键不重复出现。
- `to_dedupe_scope()` → `"{platform}:{account_id|*}:{scope|*}"` —— 去重作用域前缀。

### 3.2 RouteKeyFactory（route_key_factory.py）

- 正式字段：消息**顶层** `account_id` / `scope`，仅接受字符串或 None（其他类型抛错），
  空串归一为 None，且存在时**阻断**旧字段回退。
- 旧格式兼容链（顶层无 account_id 时按序回退，首个非空字符串胜出）：
  - account_id 候选键：`platform_io_account_id` → `account_id` → `self_id` → `bot_account`
  - scope 候选键：`platform_io_scope` → `route_scope` → `adapter_scope` → `connection_id`
  - 查找顺序：message_dict → message_info → message_info.additional_config → legacy_metadata

### 3.3 RouteTable（routing.py）

- `bindings: Dict[RouteKey, Dict[driver_id, RouteBinding]]`，binding 带 `priority`。
- `resolve_bindings()`：沿 `resolution_order()` 逐键取绑定，按 `driver_id` 去重，
  结果按 priority 降序。不裁决唯一 owner。

### 3.4 MessageDeduplicator（dedupe.py）

- 参数：`ttl_seconds=300.0`，`max_entries=10000`（非正即抛错）。
- `mark_seen(key) -> bool`：true=窗口内首次（放行），false=重复（拦截）。
- 实现：dict 存 `key→过期时刻`，小顶堆按过期时刻懒清理；同键重推后旧堆节点失效需跳过；
  容量满时淘汰最早过期的有效键。时钟为 monotonic。

## 5. Wire 协议（Adapter ⇄ Core，2026-11-03 提取完毕）

**传输 = Socket.IO**（python-socketio；不是裸 WS，TCP 模式才用 crypto.py 加密帧，WS 模式不涉及）：

| 项 | 值 | 来源 |
|---|---|---|
| 服务端 | `maim_message.api.MessageServer`（ws 模式，aiohttp/uvicorn） | api.py |
| 路径 | `/ws`（可配 socketio_path） | ws_connection.py |
| 心跳 | ping_interval 25s / ping_timeout 20s | ws_connection.py |
| 消息上限 | 100MB | ws_connection.py |
| 客户端握手头 | `platform: <platform_name>`（服务端按此路由，每平台一条连接）、`Authorization: <token>`（服务端 enable_token 时校验） | ws_connection.py |
| 事件名 | 只有一个 `"message"` | ws_connection.py |
| 入站载荷 | MessageBase 字典原样 JSON | ws_connection.py |
| 自定义消息信封 | `{platform, message_type_name, content, timestamp, is_custom_message: true}` | ws_connection.py |
| 客户端重连 | 无限次，1s→10s 退避，随机因子 0.5，polling→websocket 升级 | ws_connection.py |
| 服务端默认端口 | MessageServer 默认 18000；README 部署示例 8000（.env 决定） | api.py / adapter README |

**Rust 影响**：`upstream.rs` 需要 socket.io **客户端**（连 Core 的服务端），用
`rust-socketio` crate 或手写最小 engine.io 客户端；crypto.py 帧协议不需要实现。

**NapCat 侧（gateway ⇄ NapCat，方向已确认）**：Adapter 是 WS **客户端**连 NapCat
正向 WS：`Authorization: Bearer <token>` 头、可配心跳间隔；API 动作
`{"action", "params", "echo"}` 用 echo 关联响应；重连退避、动作超时均可配。
与骨架 `onebot/client.rs` 方向一致，需补 token 头与 echo 动作通道。

## 6. Core 入站处理链（bot.py `message_process`）

group_id/user_id 强转 str → `MessageBase.from_dict` →
`RouteKeyFactory.from_message_dict`（platform 从解析结果回填）→
`SessionMessage.from_maim_message`。
回执回填走自定义消息 `{type:"echo", echo, actual_id}`（`echo_message_process`）。

## 7. 两种载荷形态（重要）

**7.1 MessageBase**（socketio 独立进程路径，`MessageBase.to_dict`）——见 §2。

**7.2 MessageDict**（插件路径，Adapter `build_message_dict` 的产物）：

```
{
  message_id: str,            # payload.message_id，缺失时 napcat-<uuid4>
  timestamp: str(float),      # 注意是字符串！
  platform: "qq",
  message_info: {
    user_info:   { user_id, user_nickname, user_cardname? },
    group_info?: { group_id, group_name },   # 仅群消息
    additional_config: {
      self_id, napcat_message_type,          # group|private
      platform_io_target_group_id | platform_io_target_user_id,
      platform_card_payloads?                # json 卡片元数据
    }
  },
  raw_message: [ Segment ],                  # 见下
  is_mentioned, is_at, is_emoji: false, is_picture: false,
  is_command, is_notify: false,
  session_id: "", processed_plain_text, display_message
}
```

Segment（raw_message 元素）：`{type, data}`，媒体段额外带
`hash`、`binary_data_base64`（base64 内联，入站图片/语音会下载二进制）；
`at` 段 data 为 `{target_user_id, target_user_nickname?, target_user_cardname?}`；
`reply` 段 data 为 `{target_message_id, target_message_content?, ...发送者信息}`；
`forward` 段 data 为节点数组 `[{user_id?, user_nickname, user_cardname?, message_id, content:[Segment]}]`。

## 8. 转换规则（Adapter codecs，2026-11-03 提取）

### 8.1 入站（OneBot 段 → Host 段，codecs/inbound/message_codec.py）

| OneBot 段 | Host 段 | 说明 |
|---|---|---|
| text | text(data=原文) | 空文本跳过 |
| at | at(data={target_user_id,...}) | 需查群成员/陌生人信息补昵称；qq==self_id 置 is_at |
| reply | reply(data={target_message_id,...}) | 调 get_msg 补被引消息预览与发送者 |
| face | text(QQ_FACE 表或"[表情]") | 直接转文本 |
| image | image/emoji + hash + base64 | sub_type∉{0,4,9} 视为表情；下载 url 二进制，失败降级 [image]/[emoji] |
| record | voice + hash + base64 | 调 get_record 取 base64，失败降级 [voice] |
| video | text("[视频] 文件:x，大小:y") | |
| file | text("[文件] 名，大小，链接") | |
| json | （卡片混入，见 cards.py）/ 元数据入 platform_card_payloads | |
| forward | forward(data=节点数组) | 先取内联 messages/content，否则 get_forward_msg(id)；节点 content 递归转换 |
| xml/share | text("[xml]"/"[share]") | |
| 未知段 | 忽略 | |

### 8.2 出站（Host 段 → OneBot 动作，codecs/outbound/）

`build_outbound_action`：
1. 目标解析：群 → `group_info.group_id ?? additional_config.platform_io_target_group_id`；
   私聊 → `platform_io_target_user_id ?? target_user_id ?? route.target_user_id`（缺失抛错）。
2. 含 forward 段 → `send_group_forward_msg` / `send_private_forward_msg`，
   普通段按连续段打包为 node（name="MaiBot", uin=additional_config.self_id）。
3. 否则 → `send_group_msg` / `send_private_msg`，message 为段数组。

段映射（segment_encoder.py）：text→text；at→at(qq)；reply→reply(id)；
image/emoji→image(base64://, sub_type 0/1)；voice/voiceurl→record；
face→face(id)；imageurl/videourl/voiceurl→url 媒体段；video→video(base64 或 url)；
file→file（file/path/url 归一化：非 base64://、file://、http(s):// 前缀一律加 file://）；
music→music(type∈{163,qq} 默认 163, id)；forward→node 列表；
dict→按内嵌 type 分发；未知类型→`[unsupported:xxx]` 占位文本。
空数据段有专门的降级占位（[image]/[voice]/[emoji]...），整个列表为空时兜底一个空 text 段。

## 9. 架构分叉与 P1 目标选择（待定，需用户拍板）

当前 main 分支的 Adapter **同时是 MaiBot 插件**（maibot_sdk `@MessageGateway`，
in-process 走 PluginPlatformDriver RPC，载荷为 §7.2 MessageDict）。独立 socketio
进程模式（载荷 §7.1 MessageBase）可能只保留在 stable release。

- **方案 A（推荐）**：钉住 stable release，Rust 网关替换独立 Adapter 进程，
  speak socket.io client（§5 已完备），MessageBase 载荷。
- **方案 B**：跟进 main 插件架构，需实现插件 RPC 协议（尚未提取，工作量大）。

## 10. Rust 映射现状

| 契约 | Rust 实现 | 状态 |
|---|---|---|
| OneBot 事件信封 | `onebot/event.rs` | ✅ v1（flatten 容忍未知字段） |
| MessageDeduplicator | `dedupe.rs` | ✅ 已逐语义对齐 + 单测 |
| RouteKey / RouteTable / RouteKeyFactory | `routing.rs` | ✅ 已逐语义对齐 + 单测 |
| maim_message MessageBase | `message.rs` | ✅ serde 模型 + 单测（FormatInfo 默认值待对照源码复核） |
| 入站 OneBot→MessageBase 转换 | `inbound.rs` | ✅ v1 同步子集 + 单测（5 项增强待办见 README） |
| 出站 Host 段→OneBot 动作 | `outbound.rs` | ✅ 全量对齐 + 单测 |
| socket.io client（upstream.rs） | `upstream.rs` | ✅ v1（rust-socketio，编译时可能需按版本微调） |
| NapCat 双向 WS（token 头/动作写任务/重连） | `onebot/client.rs` | ✅ v1 |
| 端到端管线 | `main.rs` | ✅ v1 已接通（去重键 = driver_id:message_id） |

## 11. 下一步

1. `cargo test` + `cargo clippy` 首轮编译修复（本仓库代码均未在本地编译过）。
2. 真实环境联调：NapCat + Core(stable) 跑通收发。
3. 补齐入站增强（at 昵称 / reply 预览 / 图片语音 base64 / forward 查询）——需要
   先实现 NapCat query client（outbound 动作通道 + echo 关联）。
4. 出站追踪（echo 回执关联发送方）。
5. baseline 压测对比（P0 验收）。
