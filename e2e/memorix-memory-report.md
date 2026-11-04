# A_memorix 内存实测报告（真实 v1.3.19）

## Core 基线
```
CORE_BASELINE_RSS_MB=36.7031
CORE_RSS_AFTER_90S_MB=36.7031
```

## 灌向量实验
```
[38;5;117m11-05 05:57:15[0m [38;5;154m[配置][0m [38;5;154mMaiCore 当前版本: 1.3.19[0m
[38;5;117m11-05 05:57:15[0m [38;5;154m[配置][0m [38;5;154m正在品鉴配置文件...[0m
[33m11-05 05:57:15[0m [38;5;154m[配置][0m [38;5;154m未配置视觉识图模型，部分图片理解可能受限，请在webui或model_config中配置[0m
[38;5;117m11-05 05:57:15[0m [38;5;154m[配置][0m [38;5;154m非常的新鲜，非常的美味！[0m
VectorStore.__init__: (self, dimension: int, quantization_type: src.A_memorix.core.utils.quantization.QuantizationType = <QuantizationType.INT8: 'int8'>, index_type: str = 'sq8', data_dir: Union[str, pathlib.Path, NoneType] = None, use_mmap: bool = True, buffer_size: int = 1024)
module classes: ['Any', 'Collection', 'Counter', 'Dict', 'HAS_FAISS', 'Iterator', 'List', 'Optional', 'Path', 'QuantizationType']
CONSTRUCT FAILED: ["{'base_dir': '/tmp/mem_exp_data'}: TypeError: VectorStore.__init__() got an unexpected keyword argument 'base_dir'", "{'storage_dir': '/tmp/mem_exp_data'}: TypeError: VectorStore.__init__() got an unexpected keyword argument 'storage_dir'", "{'data_dir': '/tmp/mem_exp_data'}: TypeError: VectorStore.__init__() missing 1 required positional argument: 'dimension'", "{'config': None, 'base_dir': '/tmp/mem_exp_data'}: TypeError: VectorStore.__init__() got an unexpected keyword argument 'config'", "{}: TypeError: VectorStore.__init__() missing 1 required positional argument: 'dimension'"]
```

## core.log 尾
```
[38;5;117m11-05 05:57:12[0m [shutdown_diagnostics] 关闭步骤结束: step=停止异步任务管理器任务 耗时=0.002s outcome=completed
[38;5;117m11-05 05:57:12[0m [1;38;5;15m[主程序][0m [1;38;5;15m正在取消 1 个剩余任务...[0m
[38;5;117m11-05 05:57:12[0m [shutdown_diagnostics] 关闭步骤开始: step=取消剩余任务
[38;5;117m11-05 05:57:12[0m [shutdown_diagnostics] 关闭步骤结束: step=取消剩余任务 耗时=0.000s outcome=completed
[38;5;117m11-05 05:57:12[0m [1;38;5;15m[主程序][0m [1;38;5;15m所有剩余任务已成功取消[0m
[38;5;117m11-05 05:57:12[0m [1;38;5;15m[主程序][0m [1;38;5;15m麦麦优雅关闭完成[0m
[主程序] 事件循环已关闭
[logger] 正在关闭日志系统...
[logger] 日志系统已关闭
[主程序] 准备退出...
```
