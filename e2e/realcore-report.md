# 真 Core v1.3.19 联调报告（自动采集）

## Core 日志（末 60 行）
```
/tmp/corevenv/bin/python: can't open file '/tmp/bot.py': [Errno 2] No such file or directory
```
## Mock OneBot 日志
```
MOCK-ONEBOT: listening
```
## 进程状态
```
kimi        7942  0.1  0.6  31172 26068 ?        S    22:50   0:00 python3 /tmp/mock_onebot.py
```
## 网关stderr
```
=== STAGE report ===
+ echo '# 真 Core v1.3.19 联调报告（自动采集）'
+ echo
+ echo '## Core 日志（末 60 行）'
+ echo '```'
+ tail -60 /tmp/core.log
+ echo '```'
+ echo '## Mock OneBot 日志'
+ echo '```'
+ cat /tmp/mock_onebot.log
+ echo '```'
+ echo '## 进程状态'
+ echo '```'
+ ps aux
+ grep -E 'bot.py|maibot-gateway'
+ grep -v grep
+ echo '```'
+ echo '## 网关stderr'
+ echo '```'
+ tail -20 /tmp/realcore.log
```
