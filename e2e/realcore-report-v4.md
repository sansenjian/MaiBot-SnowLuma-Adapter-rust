# 真 Core v1.3.19 联调报告 v4（UDS 修复后）

## core.log 末 100 行
```
[33m11-05 00:33:20[0m [38;5;154m[配置][0m [38;5;154m未配置视觉识图模型，部分图片理解可能受限，请在webui或model_config中配置[0m
[38;5;117m11-05 00:33:20[0m [38;5;154m[配置][0m [38;5;154m非常的新鲜，非常的美味！[0m

══════════════════════════════════════════════════════════════════════
  ★ MaiBot - 开源 AI 拟人 Harness ★
──────────────────────────────────────────────────────────────────────
  本项目是免费的开源软件，基于 GPL-3.0 协议发布，如果有人向你「出售本软件」，你被骗了！

  官方仓库: https://github.com/Mai-with-u/MaiBot 
  官方文档: https://docs.mai-mai.org 
  官方群聊: 571780722
──────────────────────────────────────────────────────────────────────
  将本软件作为「商品」倒卖、隐瞒开源性质均违反协议！
══════════════════════════════════════════════════════════════════════

[38;5;117m11-05 00:33:20[0m [1;38;5;15m[主程序][0m [1;38;5;15m使用须知见 EULA.md，隐私说明见 PRIVACY.md。[0m
多年以后，面对AI行刑队，张三将会回想起他2023年在会议上讨论人工智能的那个下午
[38;5;117m11-05 00:33:20[0m [1;38;5;15m[主程序][0m [1;38;5;15m正在唤醒 麦麦......[0m
[38;5;117m11-05 00:33:21[0m [38;5;214m[表情包][0m [38;5;214m启动表情包管理器[0m
[33m11-05 00:33:22[0m [38;5;141m[本地存储][0m [38;5;141m啊咧？记事本不存在，正在创建新的记事本......[0m
[38;5;117m11-05 00:33:22[0m [38;5;141m[本地存储][0m [38;5;141m记事本创建成功！[0m
[38;5;117m11-05 00:33:22[0m [38;5;69m[HTTP服务][0m [38;5;69mHTTP 服务器已在独立线程中启动[0m
[38;5;117m11-05 00:33:22[0m [A-Memorix宿主服务] A_Memorix 未启用，跳过长期记忆运行时初始化
[38;5;117m11-05 00:33:22[0m [38;5;208m[核心插件][0m [38;5;208m开始拉起 Runner 子进程[0m
[38;5;117m11-05 00:33:22[0m [38;5;177m[扩展插件][0m [38;5;177m开始拉起 Runner 子进程[0m
[33m11-05 00:33:22[0m [startup_diagnostics] 诊断阶段结束: stage=plugin_runtime.start_runners pid=7117 thread=MainThread 耗时=0.025s outcome=failed
[31m11-05 00:33:22[0m [38;5;166m[IPC插件系统][0m [38;5;166m插件运行时启动失败: [Errno 2] No such file or directory: '/tmp/ws/target/release/maibot-gateway'[0m
Traceback (most recent call last):
  File "/tmp/MaiBot/src/plugin_runtime/integration.py", line 921, in start
    started_supervisors = await self._start_supervisors(builtin_dirs, third_party_dirs)
                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/plugin_runtime/integration.py", line 817, in _start_supervisors
    raise result
  File "/tmp/MaiBot/src/plugin_runtime/host/supervisor.py", line 613, in start
    await self._spawn_runner()
  File "/tmp/MaiBot/src/plugin_runtime/host/supervisor.py", line 1971, in _spawn_runner
    self._runner_process = await asyncio.create_subprocess_exec(
                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/asyncio/subprocess.py", line 224, in create_subprocess_exec
    transport, protocol = await loop.subprocess_exec(
                          ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/asyncio/base_events.py", line 1756, in subprocess_exec
    transport = await self._make_subprocess_transport(
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/asyncio/unix_events.py", line 211, in _make_subprocess_transport
    transp = _UnixSubprocessTransport(self, protocol, args, shell,
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/asyncio/base_subprocess.py", line 36, in __init__
    self._start(args=args, shell=shell, stdin=stdin, stdout=stdout,
  File "/usr/local/lib/python3.12/asyncio/unix_events.py", line 820, in _start
    self._proc = subprocess.Popen(
                 ^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/subprocess.py", line 1026, in __init__
    self._execute_child(args, executable, preexec_fn, close_fds,
  File "/usr/local/lib/python3.12/subprocess.py", line 1955, in _execute_child
    raise child_exception_type(errno_num, err_msg, err_filename)
FileNotFoundError: [Errno 2] No such file or directory: '/tmp/ws/target/release/maibot-gateway'
[38;5;117m11-05 00:33:23[0m [38;5;214m[表情包][0m [38;5;214m[数据库] 成功加载 0 个已注册表情包，清理破损注册记录 0 条，候选记录=0，查询=0.003s，校验构建=0.000s，事务收尾=0.000s，总耗时=0.004s[0m
[38;5;117m11-05 00:33:23[0m [1;38;5;15m[主程序][0m [1;38;5;15m表情包管理器初始化成功[0m
[38;5;117m11-05 00:33:23[0m [1;38;5;15m[主程序][0m [1;38;5;15m聊天管理器初始化成功[0m
[38;5;117m11-05 00:33:23[0m [1;38;5;15m[主程序][0m [1;38;5;15m初始化完成，神经元放电 2513 次[0m
[38;5;117m11-05 00:33:23[0m [38;5;242m[远程服务][0m [38;5;242m正在向遥测服务端请求UUID...[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 起点查询=0.03s[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m正在收集统计数据...[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 增量汇总=0.03s[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 多周期统计=0.06s[0m
[38;5;117m11-05 00:33:23[0m [38;5;242m[远程服务][0m [38;5;242m成功获取UUID: 8367eb66-7591-3a7a-b44c-bfd2d1939425[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m统计数据收集完成，首页缓存刷新耗时=0.54s[0m
[38;5;117m11-05 00:33:23[0m [38;5;129m[麦麦统计][0m [38;5;129m
------------------------------------------------------------------------------------
最近1小时统计 · 2026-11-05 00:33:23
在线 0分钟0秒  消息 0  回复 0  请求 0
Token 0 (输入 0 / 输出 0)  缓存命中 N/A
花费 0.00¥






------------------------------------------------------------------------------------
[0m
[38;5;117m11-05 00:33:24[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 图表与指标趋势=0.01s[0m
[38;5;117m11-05 00:33:24[0m [38;5;129m[麦麦统计][0m [38;5;129m统计数据输出完成，报表输出耗时=0.01s，总耗时=0.67s[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[WebUI][0m [38;5;69mWebUI 配置文件不存在，正在创建: /tmp/MaiBot/data/webui.json[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[WebUI][0m [38;5;69mWebUI 配置已保存到: /tmp/MaiBot/data/webui.json[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[WebUI][0m [38;5;69m新的 WebUI Token 已生成: 439a3289...[0m
[38;5;117m11-05 00:33:24[0m [38;5;68m[WebUI应用][0m [38;5;68m🔑 WebUI 登录 Token: 439a3289ff42ab778108bf34ed05ea12f4f9388b0b2838cada340aaedd7645a9[0m
[38;5;117m11-05 00:33:24[0m [38;5;68m[WebUI应用][0m [38;5;68m💡 请使用此 Token 登录 WebUI[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[HTTP服务][0m [38;5;69m🌐 HTTP 服务器启动中...[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[HTTP服务][0m [38;5;69m🌐 访问地址: http://127.0.0.1:8001[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[HTTP服务][0m [38;5;69m🌐 访问地址: http://[::1]:8001[0m
[38;5;117m11-05 00:33:24[0m [38;5;69m[HTTP服务][0m [38;5;69m🔗 HTTP 已绑定到多个地址[0m
[38;5;117m11-05 00:33:24[0m [1;38;5;15m[主程序][0m [1;38;5;15m全部系统初始化完成，麦麦 已成功唤醒[0m
[33m11-05 00:33:24[0m [1;38;5;15m[主程序][0m [1;38;5;15m终端输入已配置为启用，但当前 stdin 不是交互式终端，已跳过[0m
[38;5;117m11-05 00:33:24[0m [38;5;140m[MaiM消息][0m [38;5;140mWebSocket 服务已挂载至外部 FastAPI 应用，仅注册路由[0m
[38;5;117m11-05 00:33:24[0m [图片路径维护] 图片路径规整一次性维护开始：从 images.id > 0 继续，已检查 0 条，转换 0 条，丢弃 0 条，剩余约 0 条，批量大小 100
[?25l
图片路径规整进度 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 0/1 -- 条/s 0:00:00 -:--:--[38;5;117m11-05 00:33:24[0m [图片路径维护] 图片路径规整一次性维护完成：累计检查 0 条，转换 0 条，丢弃 0 条，跳过 0 条，最后处理 id=0

图片路径规整进度 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 1/1 -- 条/s 0:00:00 0:00:00
[?25h```

## mock onebot
```
MOCK-ONEBOT: listening
```

## 进程
```
kimi        7107  0.1  0.6  31172 25932 ?        S    00:33   0:00 python3 /tmp/mock_onebot.py
kimi        7114  0.3  0.9 119872 37848 ?        Sl   00:33   0:00 /tmp/corevenv/bin/python /tmp/MaiBot/bot.py
```

## Core 中网关/插件相关行
```
[38;5;117m11-05 00:33:22[0m [38;5;208m[核心插件][0m [38;5;208m开始拉起 Runner 子进程[0m
[38;5;117m11-05 00:33:22[0m [38;5;177m[扩展插件][0m [38;5;177m开始拉起 Runner 子进程[0m
[33m11-05 00:33:22[0m [startup_diagnostics] 诊断阶段结束: stage=plugin_runtime.start_runners pid=7117 thread=MainThread 耗时=0.025s outcome=failed
[31m11-05 00:33:22[0m [38;5;166m[IPC插件系统][0m [38;5;166m插件运行时启动失败: [Errno 2] No such file or directory: '/tmp/ws/target/release/maibot-gateway'[0m
  File "/tmp/MaiBot/src/plugin_runtime/integration.py", line 921, in start
  File "/tmp/MaiBot/src/plugin_runtime/integration.py", line 817, in _start_supervisors
  File "/tmp/MaiBot/src/plugin_runtime/host/supervisor.py", line 613, in start
    await self._spawn_runner()
  File "/tmp/MaiBot/src/plugin_runtime/host/supervisor.py", line 1971, in _spawn_runner
    self._runner_process = await asyncio.create_subprocess_exec(
FileNotFoundError: [Errno 2] No such file or directory: '/tmp/ws/target/release/maibot-gateway'
在线 0分钟0秒  消息 0  回复 0  请求 0
[38;5;117m11-05 00:33:24[0m [38;5;140m[MaiM消息][0m [38;5;140mWebSocket 服务已挂载至外部 FastAPI 应用，仅注册路由[0m
```
