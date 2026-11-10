# fuyubot（Bun 薄 SDK）

fuyubot 脚本插件的 JS/TS 薄 SDK。`bun add fuyubot` 之后：

```ts
import { on, run } from "fuyubot";

on("updateNewMessage", (update) => {
  console.log(update.message.id);
});

run();
```

它只是原生桥 `fuyubot_sdk_native` 的 `bun:ffi` 封装；协议细节全在 Rust 里。原生桥由
`fuyubot-script` 插件提供，缺省路径 `<bot>/plugins/fuyubot.script/`，也可用环境变量
`FUYUBOT_NATIVE_LIB` 指定。

文档：<https://catmoecircle.github.io/FuyuBot-docs/>
