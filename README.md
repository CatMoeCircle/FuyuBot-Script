# FuyuBot-Script

fuyubot 的**脚本插件宿主**：一个中间插件（`fuyubot_script`），同时托管 Python（uv）与
JS/TS（bun）脚本。每个脚本是独立子进程，坏了不连累宿主；宿主只 dlopen 这一个受信任的
dll。

- 运行时本体：[Fuyu-Bot](https://github.com/CatMoeCircle/Fuyu-Bot)
- 文档与示例：<https://catmoecircle.github.io/FuyuBot-docs/>

## 结构

| 路径 | 作用 |
| --- | --- |
| `crates/fuyubot-script` | **唯一的插件 dll**：合并原 `python-host` / `js-host`，按语言各建一个 `ScriptHost` |
| `crates/fuyubot-sdk-host` | 宿主侧共享逻辑（扫 `plugins/scripts/`、`uv sync` / `bun install`、起进程、stdio 桥、子作用域） |
| `crates/fuyubot-sdk-native` | 原生桥（脚本进程里被 ctypes / bun:ffi 加载） |
| `packages/python` | PyPI 包 `fuyubot`（薄 SDK） |
| `packages/js` | npm 包 `fuyubot`（薄 SDK，Bun ≥ 1.3.1） |

没有任何 crate 发布到 crates.io：这里产出的都是**预编译资产**。唯一的外部依赖是运行时
仓库发出的 `fuyubot-abi`。

## 装插件

从 Release 下载 `fuyubot-script-<OS>-<平台>.zip`，解压后把 `plugins/fuyubot.script/`
整个放进机器人的 `plugins/` 目录。重启 `fuyubot` 即生效。

脚本放在 `<bot>/plugins/scripts/<脚本目录>/`：

```text
plugins/
├── fuyubot.script/            ← 本插件
│   ├── plugin.json
│   └── fuyubot_script.dll
└── scripts/
    └── weather/
        ├── pyproject.toml     ← [tool.fuyubot] name / entry
        └── main.py
```

## 写脚本

```bash
pip install fuyubot    # Python
bun add fuyubot        # JS / TS
```

```python
from fuyubot import on, run

@on("updateNewMessage")
def handle(update):
    print(update["message"]["id"])

run()
```

薄 SDK 会加载原生桥。缺省时按发行布局找 `<bot>/plugins/fuyubot.script/fuyubot_script.dll`；
装在别处就设 `FUYUBOT_NATIVE_LIB` 指向那份 dll（本插件起脚本进程时会自动带上这个变量）。

## 构建

```bash
cargo build --workspace
cargo test  --workspace
cargo fmt --all --check
```

发布：tag `script-v*` 触发 `.github/workflows/release-script.yml`，产
`fuyubot-script-<os>-<arch>.zip`；两个薄 SDK 由 `.github/workflows/publish-packages.yml`
手动发到 PyPI / npm。
