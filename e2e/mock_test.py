#!/usr/bin/env python3
"""maibot-gateway 端到端集成测试 v3：+ invoke 串行回执与 message_id_echo 回调校验。"""
import asyncio, json, struct, sys
import msgpack
import websockets

TOKEN = "test-token"
RESULTS = []
def check(name, cond, detail=""):
    RESULTS.append((name, bool(cond)))
    print(("PASS" if cond else "FAIL"), name, "" if cond else detail, flush=True)

onebot_conn = None
sent_actions = []

async def onebot_handler(ws):
    global onebot_conn
    auth = ws.request.headers.get("Authorization", "")
    check("onebot-auth-bearer", auth == f"Bearer {TOKEN}", auth)
    url = str(ws.request.path)
    check("onebot-access-token-param", "access_token=" + TOKEN in url, url)
    onebot_conn = ws
    async for raw in ws:
        frame = json.loads(raw)
        echo, action = frame.get("echo"), frame.get("action", "")
        if action == "send_msg":
            sent_actions.append(frame)
            await ws.send(json.dumps({"status": "ok", "retcode": 0,
                                      "data": {"message_id": 999001}, "echo": echo}))
        elif action == "get_group_member_info":
            await ws.send(json.dumps({"status": "ok", "retcode": 0,
                                      "data": {"user_id": frame["params"]["user_id"],
                                               "nickname": "昵称同学", "card": "群名片X"}, "echo": echo}))
        elif action == "get_msg":
            await ws.send(json.dumps({"status": "ok", "retcode": 0,
                                      "data": {"message": [{"type": "text", "data": {"text": "被引内容"}}],
                                               "sender": {"user_id": 555, "nickname": "原发言人"}}, "echo": echo}))
        elif action == "get_record":
            await ws.send(json.dumps({"status": "ok", "retcode": 0,
                                      "data": {"base64": "dmlvaWNl"}, "echo": echo}))
        else:
            await ws.send(json.dumps({"status": "ok", "retcode": 0, "data": None, "echo": echo}))

async def inject_onebot_event():
    event = {
        "time": 1762200000.0, "self_id": 100, "post_type": "message",
        "message_type": "group", "message_id": 12345, "user_id": 888,
        "group_id": 777, "group_name": "测试群",
        "sender": {"nickname": "小明", "card": "群名片小明"},
        "message": [
            {"type": "text", "data": {"text": "hello bot"}},
            {"type": "at", "data": {"qq": "100"}},
            {"type": "reply", "data": {"id": "4444"}},
        ],
    }
    await onebot_conn.send(json.dumps(event))

host_events = {"hello": None, "register": None, "ready": None, "route": None, "invoke_resp": None}
host_writer = None

async def host_handler(reader, writer):
    global host_writer
    host_writer = writer
    buf = b""
    async def read_frame():
        nonlocal buf
        while True:
            if len(buf) >= 4:
                (n,) = struct.unpack("!I", buf[:4])
                if len(buf) >= 4 + n:
                    payload = buf[4:4 + n]
                    buf = buf[4 + n:]
                    return msgpack.unpackb(payload, raw=False)
            chunk = await reader.read(65536)
            if not chunk:
                return None
            buf += chunk

    while True:
        env = await read_frame()
        if env is None:
            return
        mt, mid, method = env["message_type"], env["request_id"], env.get("method", "")
        payload = env.get("payload") or {}
        if mt == "response":
            if mid == 9001:
                host_events["invoke_resp"] = env.get("payload")
            continue
        if method == "runner.hello":
            host_events["hello"] = payload
            out_payload = {"accepted": True, "host_version": "1.0.0-test", "reason": ""}
        elif method == "plugin.register_components":
            host_events["register"] = payload
            out_payload = {}
        elif method == "runner.ready":
            host_events["ready"] = payload
            out_payload = {}
        elif method == "host.route_message":
            host_events["route"] = payload
            out_payload = {}
        else:
            data = struct.pack("!I", len(p := msgpack.packb(
                {"protocol_version": "1.0.0", "request_id": mid, "message_type": "response",
                 "method": method, "plugin_id": "", "timestamp_ms": 0, "timeout_ms": 30000,
                 "payload": None, "error": {"code": "E_UNKNOWN", "message": "unexpected", "details": {}}},
                use_bin_type=True))) + p
            writer.write(data)
            await writer.drain()
            continue
        data = struct.pack("!I", len(p := msgpack.packb(
            {"protocol_version": "1.0.0", "request_id": mid, "message_type": "response",
             "method": method, "plugin_id": "", "timestamp_ms": 0, "timeout_ms": 30000,
             "payload": out_payload}, use_bin_type=True))) + p
        writer.write(data)
        await writer.drain()

async def host_invoke_outbound():
    msg = {
        "message_id": "out-1",
        "message_info": {
            "additional_config": {"platform_io_target_group_id": "777"},
            "user_info": {"user_id": "1", "user_nickname": "bot"},
        },
        "raw_message": [{"type": "text", "data": "自动回复"}],
    }
    req = {"protocol_version": "1.0.0", "request_id": 9001, "message_type": "request",
           "method": "plugin.invoke_message_gateway", "plugin_id": "maibot_gateway_rust",
           "timestamp_ms": 0, "timeout_ms": 30000,
           "payload": {"component_name": "napcat_gateway",
                       "args": {"message": msg, "route": {}, "metadata": {}}}}
    data = struct.pack("!I", len(p := msgpack.packb(req, use_bin_type=True))) + p
    host_writer.write(data)
    await host_writer.drain()
    for _ in range(100):
        await asyncio.sleep(0.1)
        if host_events["invoke_resp"] is not None:
            return {"payload": host_events["invoke_resp"]}
    return None

async def main():
    async with websockets.serve(onebot_handler, "127.0.0.1", 3001, ping_interval=None):
        srv = await asyncio.start_server(host_handler, "127.0.0.1", 9500)
        for _ in range(100):
            await asyncio.sleep(0.1)
            if host_events["hello"] and host_events["register"] and host_events["ready"]:
                break
        check("handshake-hello", host_events["hello"] is not None)
        check("register-components", host_events["register"] is not None)
        check("runner-ready", host_events["ready"] is not None)

        await inject_onebot_event()
        for _ in range(100):
            await asyncio.sleep(0.1)
            if host_events["route"]:
                break
        check("route-message-received", host_events["route"] is not None)
        if host_events["route"]:
            p = host_events["route"]
            m = p["message"]
            check("routed-dedupe-key", p.get("dedupe_key") == "legacy.send.qq:12345", str(p.get("dedupe_key")))
            check("dict-timestamp-is-str", isinstance(m.get("timestamp"), str), repr(m.get("timestamp")))
            check("dict-self-id", m["message_info"]["additional_config"].get("self_id") == "100")
            check("dict-msg-type", m["message_info"]["additional_config"].get("napcat_message_type") == "group")
            check("dict-target-group", m["message_info"]["additional_config"].get("platform_io_target_group_id") == "777")
            segs = m["raw_message"]
            at = next((s for s in segs if s["type"] == "at"), None)
            check("at-nickname-resolved", at is not None and at["data"].get("target_user_nickname") == "昵称同学",
                  json.dumps(at, ensure_ascii=False))
            rp = next((s for s in segs if s["type"] == "reply"), None)
            check("reply-preview", rp is not None and rp["data"].get("target_message_content") == "被引内容",
                  json.dumps(rp, ensure_ascii=False))
            check("plain-text", m.get("processed_plain_text") == "hello bot", str(m.get("processed_plain_text")))

        invoke_task = asyncio.create_task(host_invoke_outbound())
        for _ in range(100):
            await asyncio.sleep(0.1)
            if sent_actions:
                break
        check("outbound-send-msg-action", len(sent_actions) == 1, json.dumps(sent_actions, ensure_ascii=False))
        if sent_actions:
            a = sent_actions[0]
            check("outbound-action-name", a["action"] == "send_msg", a["action"])
            check("outbound-message-type", a["params"].get("message_type") == "group")
            check("outbound-group-id", a["params"].get("group_id") == "777")
            check("outbound-text", a["params"]["message"] == [{"type": "text", "data": {"text": "自动回复"}}],
                  json.dumps(a["params"]["message"], ensure_ascii=False))
        resp = await invoke_task
        check("invoke-response-success", resp is not None and (resp.get("payload") or {}).get("success") is True,
              json.dumps(resp, ensure_ascii=False)[:300])
        # v3：串行回执 + message_id_echo 回调
        pl = (resp or {}).get("payload") or {}
        check("invoke-external-id", pl.get("external_message_id") == "999001", json.dumps(pl, ensure_ascii=False)[:200])
        cbs = (pl.get("metadata") or {}).get("adapter_callbacks") or []
        echo_cb = next((c for c in cbs if c.get("name") == "message_id_echo"), None)
        check("invoke-echo-callback",
              echo_cb is not None
              and echo_cb["payload"]["content"] == {"type": "echo", "echo": "out-1", "actual_id": "999001"},
              json.dumps(cbs, ensure_ascii=False))

        failed = [n for n, ok in RESULTS if not ok]
        print("\n==== %d/%d passed ====" % (len(RESULTS) - len(failed), len(RESULTS)), flush=True)
        for n, ok in RESULTS:
            if not ok:
                print("FAILED:", n, flush=True)
        sys.exit(1 if failed else 0)

if __name__ == "__main__":
    asyncio.run(main())
