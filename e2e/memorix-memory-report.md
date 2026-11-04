# A_memorix 内存实测报告 v2（真实 v1.3.19）

## Core 基线 RSS
```
CORE_BASELINE_RSS_MB=37.6797
CORE_RSS_AFTER_90S_MB=37.6641
```

## 灌向量实验
```
[38;5;117m11-05 06:02:10[0m [38;5;154m[配置][0m [38;5;154mMaiCore 当前版本: 1.3.19[0m
[38;5;117m11-05 06:02:10[0m [38;5;154m[配置][0m [38;5;154m正在品鉴配置文件...[0m
[33m11-05 06:02:10[0m [38;5;154m[配置][0m [38;5;154m未配置视觉识图模型，部分图片理解可能受限，请在webui或model_config中配置[0m
[38;5;117m11-05 06:02:10[0m [38;5;154m[配置][0m [38;5;154m非常的新鲜，非常的美味！[0m
init: (self, dimension: int, quantization_type: src.A_memorix.core.utils.quantization.QuantizationType = <QuantizationType.INT8: 'int8'>, index_type: str = 'sq8', data_dir: Union[str, pathlib.Path, NoneType] = None, use_mmap: bool = True, buffer_size: int = 1024)
add: (self, vectors: numpy.ndarray, ids: List[str]) -> int
methods: ['DEFAULT_MIN_TRAIN', 'TRAIN_SIZE', '_flush_write_buffer', '_flush_write_buffer_unlocked', '_force_train_small_data', '_force_train_small_data_unlocked', '_train_and_replay', '_train_and_replay_unlocked', 'add', 'needs_training', 'search', 'warmup_index']
start_rss=161.2MB mmap=?
add failed: ValueError('Vector IDs must be non-empty strings')
INSERT DONE 20000 in 0.1s rss=196.8MB
calling warmup_index() ...
after warmup_index: rss=196.8MB main=0 fallback=0
calling _train_and_replay() ...
[33m11-05 06:02:12[0m [记忆向量存储] No training data available.
after _train_and_replay: rss=196.8MB main=0 fallback=0
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
[33m11-05 06:02:14[0m [记忆向量存储] Indices are empty. No data to search.
search(20): first=1.0ms avg20=0.14ms
```
