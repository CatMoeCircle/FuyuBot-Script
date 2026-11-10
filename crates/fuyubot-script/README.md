# fuyubot-script

脚本插件宿主：**一个** cdylib 同时托管 Python（uv）与 JS/TS（bun）脚本。

- 宿主进程里它是 ABI 插件（`define_plugin!(…, "fuyubot.script")`），扫 `plugins/scripts/`、
  装依赖、起脚本进程、接 stdio 协议、按脚本划子作用域；
- 脚本进程里**同一份 dll** 是原生后端（链接 `fuyubot-sdk-native` 的 `fuyubot_bridge_*`）。

服务名保持 `fuyubot.host.scripts.python` / `fuyubot.host.scripts.js`，宿主 `plugin list`
按前缀枚举，不用改运行时。

## stdio 协议（一行一个 JSON，`op` 是动作名）

**这一份是权威**：原生桥（`fuyubot-sdk-native`）实现它，宿主侧 `fuyubot-sdk-host` 照它解析。

脚本 → 宿主（脚本 stdout）：

| 行 | 作用 | 宿主回什么 |
|---|---|---|
| `{"op":"ready"}` | 告诉宿主脚本起来了 | 无 |
| `{"op":"log","level":"info","message":"…"}` | 打日志；`level` 可为 `error/warn/info/debug/trace` 或 1..5 | 无 |
| `{"op":"on","tag":1,"event":"updateNewMessage","filter":{…}}` | 订阅，`filter` 可省 | `{"op":"on","tag":1,"handle":N}` |
| `{"op":"off","handle":N}` | 退订 | 无 |
| `{"op":"invoke","tag":2,"method":"sendMessage","params":{…}}` | 发 TDLib 请求 | `{"op":"invoked","tag":2,"request_id":R}` |
| `{"op":"emit","event":"x","payload":{…}}` | 发一条自定义事件 | 无 |
| `{"op":"tool","descriptor":{…}}` | 登记工具元数据 | `{"op":"tool","tag":n,"handle":N}` |
| `{"op":"callTool","tag":n,"name":"x","input":{…}}` | 同步调用工具 | `{"op":"toolResult","tag":n,"output":{…}}` |
| `{"op":"listTools","tag":n}` | 列出全部工具元数据 | `{"op":"listTools","tag":n,"tools":[…]}` |
| `{"op":"execute","tag":n,"request":{…}}` | 同步执行不碰网络的 TDLib 方法 | `{"op":"execute","tag":n,"result":{…}}` |
| `{"op":"middleware","tag":n,"stage":N}` | 挂中间件（`stage` 0..5） | `{"op":"middleware","tag":n,"handle":N}` |
| `{"op":"wait","tag":n,"filter":{…},"timeout_ms":M}` | 阻塞等一条匹配的更新 | `{"op":"wait","tag":n,"payload":{…}}` |
| `{"op":"getService","tag":n,"name":"x"}` | 读一个服务的元数据 | `{"op":"getService","tag":n,"service":{…}}` |
| `{"op":"callService","tag":n,"name":"x","method":"m","input":{…}}` | 同步调服务方法 | `{"op":"callService","tag":n,"result":{…}}` |
| `{"op":"getConfig","tag":n,"section":"x","key":"a.b"}` | 读全局配置（`key` 可省 = 整段） | `{"op":"getConfig","tag":n,"config":{…}}` |
| `{"op":"setConfig","tag":n,"key":"a.b","value":{…}}` | 写**自己**的配置段（`key` 空 = 整段） | `{"op":"setConfig","tag":n}`；失败回 `error` |

宿主 → 脚本（脚本 stdin）：

| 行 | 作用 |
|---|---|
| `{"op":"event","event":"updateNewMessage","payload":{…}}` | 你订阅的事件来了 |
| `{"op":"middleware","handle":N,"stage":N,"payload":{…}}` | 你挂的中间件被调了 |
| 上面那几种回执 | 你给的 `tag` 原样带回，方便脚本自己配对 |

## 薄 SDK 从哪来

`packages/python` 与 `packages/js` 分别发 PyPI / npm 的 `fuyubot` 包，作者
`pip install fuyubot` / `bun add fuyubot` 即可；原生桥由本插件在起脚本进程时通过
`FUYUBOT_NATIVE_LIB` 指过去，不需要手动拷 dll。
