# fuyubot（Python 薄 SDK）

fuyubot 脚本插件的 Python 薄 SDK。`pip install fuyubot` 之后：

```python
from fuyubot import on, run

@on("updateNewMessage")
def handle(update):
    print(update["message"]["id"])

run()
```

它只是原生桥 `fuyubot_sdk_native` 的 ctypes 封装；协议细节（一行一个 JSON、订阅号、
`ready` 握手）全在 Rust 里。原生桥由 `fuyubot-script` 插件提供，缺省路径
`<bot>/plugins/fuyubot.script/`，也可用环境变量 `FUYUBOT_NATIVE_LIB` 指定。

文档：<https://catmoecircle.github.io/FuyuBot-docs/>
