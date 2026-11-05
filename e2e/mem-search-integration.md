[38;5;117m11-05 20:53:17[0m [38;5;154m[配置][0m [38;5;154mMaiCore 当前版本: 1.3.19[0m
[38;5;117m11-05 20:53:17[0m [38;5;154m[配置][0m [38;5;154m正在品鉴配置文件...[0m
[33m11-05 20:53:18[0m [38;5;154m[配置][0m [38;5;154m未配置视觉识图模型，部分图片理解可能受限，请在webui或model_config中配置[0m
[38;5;117m11-05 20:53:18[0m [38;5;154m[配置][0m [38;5;154m非常的新鲜，非常的美味！[0m
added 5000 rss=245.2MB trained=False bin_exists=True
persistence candidates: ['_copy_and_sync', '_flush_write_buffer', '_flush_write_buffer_unlocked', '_read_persisted_vectors_unlocked', '_sync_append', '_truncate_and_sync']
_copy_and_sync failed: TypeError("VectorStore._copy_and_sync() missing 2 required positional arguments:
called _flush_write_buffer()
called _flush_write_buffer_unlocked()
_read_persisted_vectors_unlocked failed: TypeError("VectorStore._read_persisted_vectors_unlocked() missing 1 required pos
_sync_append failed: TypeError("VectorStore._sync_append() missing 2 required positional arguments: '
_truncate_and_sync failed: TypeError("VectorStore._truncate_and_sync() missing 2 required positional argume
data files: ['vectors_append.json', 'vectors_ids.bin', 'vectors.bin']
REOPEN rss=216.1MB (pre 216.1) mem_fallback=NO faiss_fb_ntotal=0
[31m╭─[0m[31m────────────────────[0m[31m [0m[1;31mTraceback [0m[1;2;31m(most recent call last)[0m[31m [0m[31m─────────────────────[0m[31m─╮[0m
[31m│[0m [2m/tmp/[0m[1mmemval.py[0m:46 in <module>                                                [31m│[0m
[31m│[0m                                                                              [31m│[0m
[31m│[0m   [2m43 [0m                                                                        [31m│[0m
[31m│[0m   [2m44 [0mq = rng.standard_normal(DIM).astype(np.float32); q /= np.linalg.norm(q) [31m│[0m
[31m│[0m   [2m45 [0mt0 = time.time()                                                        [31m│[0m
[31m│[0m [31m❱ [0m46 hits = [1;4mstore2.search(q, k=[0m[1;4;94m10[0m[1;4m)[0m                                           [31m│[0m
[31m│[0m   [2m47 [0mn_hits = [96mlen[0m(hits[[94m0[0m]) [94mif[0m [96misinstance[0m(hits, [96mtuple[0m) [94melse[0m [96mlen[0m(hits)         [31m│[0m
[31m│[0m   [2m48 [0m[96mprint[0m([33mf[0m[33m"[0m[33msearch(10): [0m[33m{[0mn_hits[33m}[0m[33m hits in [0m[33m{[0m(time.time()-t0)*[94m1000[0m[33m:[0m[33m.1f[0m[33m}[0m[33mms[0m[33m"[0m, fl [31m│[0m
[31m│[0m   [2m49 [0m                                                                        [31m│[0m
[31m│[0m                                                                              [31m│[0m
[31m│[0m [2m/tmp/mb/src/A_memorix/core/storage/[0m[1mvector_store.py[0m:1790 in search            [31m│[0m
[31m│[0m                                                                              [31m│[0m
[31m│[0m   [2m1787 [0m[2m│   │   │   [0mrequested_k = [96mmax[0m([94m1[0m, [96mint[0m(k))                              [31m│[0m
[31m│[0m   [2m1788 [0m[2m│   │   │   [0m[2m# rust-memorix：mem 兜底（磁盘全量，mmap 扫描）与 faiss  [0m [31m│[0m
[31m│[0m   [2m1789 [0m[2m│   │   │   [0mmem_hits: List[Tuple[[96mint[0m, [96mfloat[0m]] = []                    [31m│[0m
[31m│[0m [31m❱ [0m1790 [2m│   │   │   [0m[94mif[0m search_index [95mis[0m [96mself[0m._fallback_index [95mand[0m [1;4;96mself[0m[1;4m._mem_fal[0m [31m│[0m
[31m│[0m   [2m1791 [0m[2m│   │   │   │   [0m[94mtry[0m:                                                  [31m│[0m
[31m│[0m   [2m1792 [0m[2m│   │   │   │   │   [0mmem_hits = [96mself[0m._mem_fallback.search(             [31m│[0m
[31m│[0m   [2m1793 [0m[2m│   │   │   │   │   │   [0mquery_local.astype(np.float32).ravel().tolist [31m│[0m
[31m╰──────────────────────────────────────────────────────────────────────────────╯[0m
[1;91mAttributeError: [0m[32m'VectorStore'[0m object has no attribute [32m'_mem_fallback'[0m
