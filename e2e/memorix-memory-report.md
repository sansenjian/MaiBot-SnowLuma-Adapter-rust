# A_memorix 内存实测报告 v3（真实 v1.3.19，2 万条 1536 维）

## Core 基线 RSS
```
CORE_RSS_MB=37.4961
```

## 实验数据
```
[38;5;117m11-05 06:04:19[0m [38;5;154m[配置][0m [38;5;154mMaiCore 当前版本: 1.3.19[0m
[38;5;117m11-05 06:04:19[0m [38;5;154m[配置][0m [38;5;154m正在品鉴配置文件...[0m
[33m11-05 06:04:19[0m [38;5;154m[配置][0m [38;5;154m未配置视觉识图模型，部分图片理解可能受限，请在webui或model_config中配置[0m
[38;5;117m11-05 06:04:19[0m [38;5;154m[配置][0m [38;5;154m非常的新鲜，非常的美味！[0m
start_rss=161.2MB mmap=?
batch 2000: rss=232.5MB main=0 fallback=2000
batch 4000: rss=268.0MB main=0 fallback=4000
batch 6000: rss=309.3MB main=0 fallback=6000
batch 8000: rss=309.3MB main=0 fallback=8000
[38;5;117m11-05 06:04:20[0m [记忆向量存储] 训练样本达到上限，开始训练...
[38;5;117m11-05 06:04:20[0m [记忆向量存储] Training Index with 10000 samples...
[38;5;117m11-05 06:04:20[0m [记忆向量存储] Replaying data from disk to populate index...
[38;5;117m11-05 06:04:20[0m [记忆向量存储] Replay successful (10000/10000). Releasing fallback index.
batch 10000: rss=475.4MB main=10000 fallback=0
batch 12000: rss=475.4MB main=12000 fallback=0
batch 14000: rss=475.4MB main=14000 fallback=0
batch 16000: rss=475.4MB main=16000 fallback=0
batch 18000: rss=475.4MB main=18000 fallback=0
batch 20000: rss=475.4MB main=20000 fallback=0
ADDED=20000 in 0.9s rss=475.4MB
warmup done in 0.0s rss=475.4MB main=20000 fallback=0
search(20): first=18.0ms avg20=1.71ms rss=475.4MB
```
