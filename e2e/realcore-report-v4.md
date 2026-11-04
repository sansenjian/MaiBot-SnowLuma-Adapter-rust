# 真 Core v1.3.19 联调报告 v4（UDS 修复后）

## core.log 末 100 行
```
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/llm_models/model_client/adapter_base.py", line 86, in await_task_with_interrupt
    return await task
           ^^^^^^^^^^
  File "/tmp/MaiBot/src/maisaka/chat_loop_service.py", line 1328, in chat_loop_step
    generation_result = await llm_chat.generate_response_with_context(
                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/services/llm_service.py", line 347, in generate_response_with_context
    result = await self._orchestrator.generate_response_with_context_async(
             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/llm_models/utils_model.py", line 523, in generate_response_with_context_async
    execution_result = await self._execute_request(
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/llm_models/utils_model.py", line 1489, in _execute_request
    raise last_exception
  File "/tmp/MaiBot/src/llm_models/utils_model.py", line 1093, in _attempt_request_on_model
    response = await client.get_response(active_request)
               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/llm_models/model_client/adapter_base.py", line 127, in get_response
    response, usage_record = await self._execute_response_request(
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/tmp/MaiBot/src/llm_models/model_client/openai_client.py", line 1766, in _execute_response_request
    raise wrapped_error from exc
src.llm_models.exceptions.RespNotOkException: API-Key错误，认证失败，请检查/config/model_list.toml中的配置是否正确

The above exception was the direct cause of the following exception:

Traceback (most recent call last):
  File "/tmp/MaiBot/src/runtime.py", line 413, in _await_shutdown_step
    await asyncio.wait_for(awaitable, timeout=_shutdown_time_left(timeout))
  File "/usr/local/lib/python3.12/asyncio/tasks.py", line 520, in wait_for
    return await fut
           ^^^^^^^^^
  File "/tmp/MaiBot/src/chat/heart_flow/heartflow_manager.py", line 84, in shutdown
    raise RuntimeError(f"{len(failures)} 个聊天运行实例未能停止") from failures[0]
RuntimeError: 1 个聊天运行实例未能停止
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=停止外接 Agent
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤结束: step=停止外接 Agent 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=保存并断开电脑
[38;5;117m11-05 00:52:39[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 增量汇总=0.01s[0m
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤结束: step=保存并断开电脑 耗时=0.002s outcome=completed
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=关闭 HTTP 服务
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤结束: step=关闭 HTTP 服务 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=停止配置监听
[38;5;117m11-05 00:52:39[0m [38;5;154m[配置][0m [38;5;154m配置文件监视器停止统计: batches=0, changes=0, ok=0, failed=0, timeout=0, cooldown_skip=0, restart=0[0m
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤结束: step=停止配置监听 耗时=0.003s outcome=completed
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=触发 ON_STOP 事件
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤结束: step=触发 ON_STOP 事件 耗时=0.003s outcome=completed
[38;5;117m11-05 00:52:39[0m [shutdown_diagnostics] 关闭步骤开始: step=停止插件运行时
[38;5;117m11-05 00:52:39[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 多周期统计=0.01s[0m
[38;5;117m11-05 00:52:40[0m [38;5;129m[麦麦统计][0m [38;5;129m统计数据收集完成，首页缓存刷新耗时=0.04s[0m
[38;5;117m11-05 00:52:40[0m [38;5;129m[麦麦统计][0m [38;5;129m
------------------------------------------------------------------------------------
最近1小时统计 · 2026-11-05 00:52:39
在线 19分钟16秒  消息 1  回复 0  请求 0
Token 0 (输入 0 / 输出 0)  缓存命中 N/A
花费 0.00¥





聊天消息（前5）:
测试群                                        1
------------------------------------------------------------------------------------
[0m
[38;5;117m11-05 00:52:40[0m [38;5;129m[麦麦统计][0m [38;5;129m统计阶段耗时: 图表与指标趋势=0.00s[0m
[38;5;117m11-05 00:52:40[0m [38;5;129m[麦麦统计][0m [38;5;129m统计数据输出完成，报表输出耗时=0.01s，总耗时=0.09s[0m
[33m11-05 00:52:41[0m [38;5;177m[扩展插件][0m [38;5;177mRunner 优雅退出超时，尝试 terminate[0m
[33m11-05 00:52:41[0m [38;5;208m[核心插件][0m [38;5;208mRunner 优雅退出超时，尝试 terminate[0m
[38;5;117m11-05 00:52:41[0m [38;5;177m[扩展插件][0m [38;5;177mRunner 退出: pid=11434 returncode=-15 reason=host_stop[0m
[38;5;117m11-05 00:52:41[0m [38;5;177m[扩展插件][0m [38;5;177m已停止[0m
[38;5;117m11-05 00:52:41[0m [38;5;208m[核心插件][0m [38;5;208mRunner 退出: pid=11431 returncode=-15 reason=host_stop[0m
[38;5;117m11-05 00:52:41[0m [38;5;208m[核心插件][0m [38;5;208m已停止[0m
[38;5;117m11-05 00:52:41[0m [38;5;166m[IPC插件系统][0m [38;5;166m插件运行时已停止[0m
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=停止插件运行时 耗时=1.520s outcome=completed
[38;5;117m11-05 00:52:41[0m [38;5;68m[图片][0m [38;5;68m图片管理器初始化完成[0m
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=等待图片描述同步
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=等待图片描述同步 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=停止记忆自动写入
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=停止记忆自动写入 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=关闭 A_Memorix 并持久化
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=关闭 A_Memorix 并持久化 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=关闭 Emoji
[38;5;117m11-05 00:52:41[0m [38;5;214m[表情包][0m [38;5;214m[关闭] Emoji 模块已注销配置热重载回调[0m
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=关闭 Emoji 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=关闭 MCP
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=关闭 MCP 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=停止异步任务管理器任务
[38;5;117m11-05 00:52:41[0m [38;5;129m[异步任务管理][0m [38;5;129m所有异步任务已停止[0m
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=停止异步任务管理器任务 耗时=0.002s outcome=completed
[38;5;117m11-05 00:52:41[0m [1;38;5;15m[主程序][0m [1;38;5;15m正在取消 1 个剩余任务...[0m
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤开始: step=取消剩余任务
[38;5;117m11-05 00:52:41[0m [shutdown_diagnostics] 关闭步骤结束: step=取消剩余任务 耗时=0.000s outcome=completed
[38;5;117m11-05 00:52:41[0m [1;38;5;15m[主程序][0m [1;38;5;15m所有剩余任务已成功取消[0m
[31m11-05 00:52:41[0m [1;38;5;15m[主程序][0m [1;38;5;15m应用优雅关闭未完成；退出状态为失败[0m
[主程序] 事件循环已关闭
[logger] 正在关闭日志系统...
[logger] 日志系统已关闭
[主程序] 准备退出...
```

## mock onebot
```
MOCK-ONEBOT: listening
MOCK-ONEBOT: connected auth=Bearer test-token
MOCK-ONEBOT: connected auth=Bearer test-token
MOCK-ONEBOT: injected
MOCK-ONEBOT: injected
connection handler failed
Traceback (most recent call last):
  File "/usr/local/lib/python3.12/site-packages/websockets/asyncio/server.py", line 376, in conn_handler
    await self.handler(connection)
  File "/tmp/mock_onebot.py", line 16, in handler
    async for raw in ws:
  File "/usr/local/lib/python3.12/site-packages/websockets/asyncio/connection.py", line 242, in __aiter__
    yield await self.recv()
          ^^^^^^^^^^^^^^^^^
  File "/usr/local/lib/python3.12/site-packages/websockets/asyncio/connection.py", line 322, in recv
    raise self.protocol.close_exc from self.recv_exc
websockets.exceptions.ConnectionClosedError: no close frame received or sent
connection handler failed
Traceback (most recent call last):
  File "/usr/local/lib/python3.12/site-packages/websockets/asyncio/server.py", line 376, in conn_handler
```

## 进程
```
kimi        7107  0.0  0.6  31500 26188 ?        S    00:33   0:00 python3 /tmp/mock_onebot.py
kimi        7114  0.0  0.9 119872 37848 ?        Sl   00:33   0:01 /tmp/corevenv/bin/python /tmp/MaiBot/bot.py
```

## Core 中网关/插件相关行
```
[38;5;117m11-05 00:52:09[0m [38;5;208m[核心插件][0m [38;5;208m开始拉起 Runner 子进程[0m
[38;5;117m11-05 00:52:09[0m [38;5;177m[扩展插件][0m [38;5;177m开始拉起 Runner 子进程[0m
[38;5;117m11-05 00:52:09[0m [38;5;208m[核心插件][0m [38;5;208mRunner 已拉起，pid=11431 耗时=0.032s returncode=None[0m
[38;5;117m11-05 00:52:09[0m [38;5;177m[扩展插件][0m [38;5;177mRunner 已拉起，pid=11434 耗时=0.001s returncode=None[0m
    raise OSError(build_port_conflict_message(service_name=service_name, host=host, port=port))
[33m11-05 00:52:12[0m [platform_io.route_key] 消息使用旧版路由归属格式，请及时更新适配器，迁移到消息顶层 account_id/scope: platform=qq, message_id=12345
[38;5;117m11-05 00:52:12[0m [38;5;172m[MaiSaka][0m [38;5;172m[测试群] 检测到提及消息，下一轮 Planner 将强制触发；消息编号=12345[0m
[38;5;117m11-05 00:52:12[0m [38;5;172m[MaiSaka][0m [38;5;172m[测试群] 消息防抖：档位=自然，规则=安静开批 2 秒；受@或提及 1 秒上限限制，剩余等待=0.981 秒，待处理=1 条，消息编号=12345[0m
[33m11-05 00:52:12[0m [platform_io.route_key] 消息使用旧版路由归属格式，请及时更新适配器，迁移到消息顶层 account_id/scope: platform=qq, message_id=12345
[38;5;117m11-05 00:52:12[0m [平台接入管理] 忽略重复入站消息: dedupe_key=gateway:maibot_gateway_rust:napcat_gateway:legacy.send.qq:12345
[38;5;117m11-05 00:52:13[0m [38;5;172m[MaiSaka][0m [38;5;172m[测试群] 已结束本次强制触发状态；触发原因=提及消息 触发消息编号=12345[0m
[38;5;117m11-05 00:52:13[0m [38;5;44m[麦麦推理引擎][0m [38;5;44m[测试群] 检测到新的提及消息（消息编号=12345），本轮直接进入 Planner。[0m
[38;5;117m11-05 00:52:13[0m [38;5;44m[麦麦推理引擎][0m [38;5;44m[测试群] 开始思考 轮次=1 历史消息=1[0m
[33m11-05 00:52:39[0m [38;5;208m[核心插件][0m [38;5;208m启动未完成：pid=11431，RPC=/tmp/maibot-plugin-11417-de9da980.sock，等待 Runner 就绪超时（30.0s）[0m
[33m11-05 00:52:39[0m [38;5;177m[扩展插件][0m [38;5;177m启动未完成：pid=11434，RPC=/tmp/maibot-plugin-11417-f73241f9.sock，等待 Runner 就绪超时（30.0s）[0m
    raise OSError(build_port_conflict_message(service_name=service_name, host=host, port=port))
在线 19分钟16秒  消息 1  回复 0  请求 0
聊天消息（前5）:
[33m11-05 00:52:41[0m [38;5;177m[扩展插件][0m [38;5;177mRunner 优雅退出超时，尝试 terminate[0m
[33m11-05 00:52:41[0m [38;5;208m[核心插件][0m [38;5;208mRunner 优雅退出超时，尝试 terminate[0m
[38;5;117m11-05 00:52:41[0m [38;5;177m[扩展插件][0m [38;5;177mRunner 退出: pid=11434 returncode=-15 reason=host_stop[0m
[38;5;117m11-05 00:52:41[0m [38;5;208m[核心插件][0m [38;5;208mRunner 退出: pid=11431 returncode=-15 reason=host_stop[0m
```
