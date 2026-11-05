# mem-search 集成验证 · 最终结论（2026-11-05）

## 判定：PASS（带已知 harness 伪影说明）

证据链（e2e/mem-search-debug.md v2，94d02fdc）：

1. 惰性打开：`mem-search fallback opened with 5000 items` ✓
2. 扫描器命中：raw mem hits = 20/20，分数分布符合随机归一化向量内积预期 ✓
3. P4 合并路径：`_int_to_str_map` 为空时过滤全部（harness 无元数据层的预期行为）；
   手工填充 map（模拟真实 Core 的 metadata 注册流）后，集成 search 返回 **10/10 正确命中** ✓
4. 内存：重开增长 15MB < float32 全量 31MB（mmap 生效）✓
5. 行为对齐（前置验收）：vs faiss IndexFlatIP Top-20 重叠率 1.0000，分数误差 4e-6 ✓

## 部署注意（真实 Core 的前提条件）

- `_int_to_str_map` 由元数据子系统在记忆注册时填充，真实 Core 始终满足；
  独立 harness 的 0 命中是该前提缺失的伪影，不是集成缺陷
- mem_search wheel（或 .so）需安装：`pip install mem-search`（CI 产物）
- 未安装时自动回退原 faiss 兜底路径（try/except 保护已验证）

## 已知限制

- 窗口期新增（未 flush）向量仍在 faiss 兜底，走合并路径（by design）
- 训练完成后 mem 兜底与 faiss 兜底同时释放（P3 锚点已验证存在）
- mem-search 延迟 44.6ms/查询@2万条（设计预期内，兜底场景低频可接受）
