//! 脚本插件宿主：**一个**中间插件同时托管 Python 与 JS/TS。
//!
//! 宿主进程里它是 ABI 插件（[`define_plugin!`] 展开出的那套导出符号 + 持有
//! [`HostApi`]）；脚本进程里**同一份 dll** 是原生后端（链接进来的 `fuyubot_bridge_*`，
//! 读写脚本自己的 stdio）。两条代码路径互不相识，见设计文档 4.1。
//!
//! 为什么合成一个：Python 与 JS 的宿主侧逻辑本来就只有 `Lang` 一个参数不同，
//! 拆成两个 dll 只是让用户多装一个文件、让商店多一条索引。这里按语言各建一个
//! [`ScriptHost`]，服务名仍是 `fuyubot.host.scripts.python` / `.js`（宿主 `plugin list`
//! 按前缀枚举，不用改）。

use std::sync::Arc;

use fuyubot_abi::{HostApi, Plugin, define_plugin};
use fuyubot_sdk_host::{HostExt, Lang, ScriptHost, scripts_dir, spawn_scripts};

struct ScriptPlugin {
    api: HostApi,
    hosts: Vec<Arc<ScriptHost>>,
}

impl Plugin for ScriptPlugin {
    fn init(api: HostApi) -> Result<Self, String> {
        Ok(Self {
            api,
            hosts: Vec::new(),
        })
    }

    fn start(&mut self, config_json: &str) -> Result<(), String> {
        let dir = scripts_dir(config_json);
        // `spawn: false`（`fuyubot plugin list` 传的）= 只发现不启动。
        let spawn = spawn_scripts(config_json);

        for lang in [Lang::Python, Lang::Js] {
            // `new` 要趁着宿主还在 `plugin_scope` 里调用：它创建的根作用域继承本插件
            // 的 owner，热加载进来的脚本靠它认祖归宗。
            let host = Arc::new(ScriptHost::new(self.api.clone(), dir.clone(), spawn));
            let outcomes = host.start_all(lang);
            for (name, outcome) in &outcomes {
                match outcome {
                    Ok(()) => self.api.info(&format!("脚本 {name} 已启动（{lang:?}）")),
                    Err(error) => self
                        .api
                        .warn(&format!("脚本 {name} 启动失败（{lang:?}）: {error}")),
                }
            }
            self.api.info(&format!(
                "fuyubot-script {lang:?} 就绪，共 {} 个脚本{}",
                host.script_count(),
                if spawn {
                    ""
                } else {
                    "（只发现模式：不装依赖、不起进程）"
                }
            ));
            // 依赖装完、进程起来之后才挂 watch，否则 `uv sync` / `bun install` 的
            // 写文件会被误判成「新增脚本」（设计文档 S3 第一条细节）。
            if spawn {
                host.watch(lang);
            }
            self.hosts.push(host);
        }
        Ok(())
    }

    fn stop(&mut self) {
        for host in self.hosts.drain(..) {
            // 显式收 watch 线程：dll 卸载之后还在它代码段里跑的线程 = 段错误。
            host.stop_watch();
            host.stop_all();
        }
    }
}

define_plugin!(ScriptPlugin, "fuyubot.script");
