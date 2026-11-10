//! `HostApi` 的 Rust 便利层。
//!
//! 原来在 `fuyubot-plugin-api`；那个 crate 已删（Rust 插件直接用 `fuyubot-abi`）。
//! 本仓库自己还要这层语法糖：脚本宿主是 Rust 写的，按 `serde_json::Value` 收发
//! 比每次手拼 JSON 字符串省事；另外 [`Subscription`] 和
//! [`wait_for_update_json`](HostExt::wait_for_update_json) 各自挡住一类容易写错的
//! 生命周期问题。
//!
//! 它**不是**对外契约：第三方 Rust 插件不需要它。

use core::ffi::{c_char, c_void};

use fuyubot_abi::{AbiError, EventCallbackFn, HostApi, LogLevel, RequestId, SubscriptionHandle};
use serde_json::Value;

/// 一条**活着的**订阅：drop 就是退订。
///
/// `HostApi::on` 返回的是裸 [`SubscriptionHandle`]；忘了 `off` 就会一直收消息，
/// 插件卸载前谁也发现不了。这个守卫把「注册 → 注销」绑成一个值。
#[must_use = "drop 掉守卫就会立刻退订"]
pub struct Subscription {
    api: HostApi,
    handle: SubscriptionHandle,
}

impl Subscription {
    /// 绑一个句柄；无效句柄不建守卫。
    pub fn new(api: HostApi, handle: SubscriptionHandle) -> Option<Self> {
        handle.is_valid().then_some(Self { api, handle })
    }

    pub fn handle(&self) -> SubscriptionHandle {
        self.handle
    }

    /// 主动退订，并吃掉守卫。
    pub fn unsubscribe(mut self) {
        self.api.off(self.handle);
        self.handle = SubscriptionHandle::INVALID;
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if self.handle.is_valid() {
            self.api.off(self.handle);
        }
    }
}

/// 把「回调收到的临时 JSON 指针」读成 [`Value`]。
///
/// # Safety
/// `payload` 必须来自宿主（ABI 契约保证它是临时 NUL 结尾字符串）。
pub unsafe fn parse_payload(payload: *const c_char) -> Value {
    // SAFETY: 由调用方保证。
    let text = unsafe { std::ffi::CStr::from_ptr(payload) }.to_string_lossy();
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// 插件作者视角的宿主能力（本仓库内部用）。
pub trait HostExt {
    /// 打一条日志（三档简写）。
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);

    /// 订阅事件，用 [`Value`] 取过滤条件；返回 drop 即退订的守卫。
    fn subscribe_json(
        &self,
        event_type: &str,
        filter: Option<Value>,
        callback: EventCallbackFn,
        user_data: *mut c_void,
    ) -> Option<Subscription>;

    /// 发一条 TDLib 请求，参数是 [`Value`]。
    fn invoke_value(&self, method: &str, params: Value) -> Result<RequestId, AbiError>;

    /// 等一条匹配的更新，并把结果当场拷成 [`Value`]（宿主的缓冲区早就失效了）。
    fn wait_for_update_json(&self, filter: Value, timeout_ms: u64) -> Option<Value>;

    /// **同步**调用工具并返回结果（工具结果回投）。
    fn call_tool_value(&self, name: &str, input: Value) -> Result<Value, AbiError>;

    /// 读全局配置的一个段 / 键，把宿主的 JSON 缓冲当场解析成 [`Value`]。
    fn config_value(&self, section: &str, key: &str) -> Option<Value>;

    /// 写**自己**那一段配置。
    fn set_config_value(&self, key: &str, value: Value) -> Result<(), AbiError>;

    /// 让宿主跑一个异步 simplify 操作。
    fn simplify_invoke_json(&self, op: Value) -> Result<RequestId, AbiError>;

    /// **同步**跑一个不碰网络的 simplify 操作（回调里安全）。
    fn simplify_invoke_result_json(&self, op: Value) -> Result<Value, AbiError>;
}

impl HostExt for HostApi {
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }

    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    fn subscribe_json(
        &self,
        event_type: &str,
        filter: Option<Value>,
        callback: EventCallbackFn,
        user_data: *mut c_void,
    ) -> Option<Subscription> {
        let filter = filter.map(|value| value.to_string());
        Subscription::new(
            self.clone(),
            self.on(event_type, filter.as_deref(), callback, user_data),
        )
    }

    fn invoke_value(&self, method: &str, params: Value) -> Result<RequestId, AbiError> {
        let params = if params.is_object() {
            params.to_string()
        } else {
            "{}".to_owned()
        };
        self.td_invoke_simplified(method, &params)
    }

    fn wait_for_update_json(&self, filter: Value, timeout_ms: u64) -> Option<Value> {
        let handle = self.wait_for_update(&filter.to_string(), timeout_ms);
        if !handle.did_match() {
            return None;
        }
        // SAFETY: 宿主保证 payload 在本线程下一次宿主调用前有效——这里立刻拷走。
        Some(unsafe { parse_payload(handle.payload_json) })
    }

    fn call_tool_value(&self, name: &str, input: Value) -> Result<Value, AbiError> {
        let text = self.call_tool_result(name, &input.to_string())?;
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|_| AbiError::BadString)
    }

    fn config_value(&self, section: &str, key: &str) -> Option<Value> {
        let text = self.get_config(section, key)?;
        serde_json::from_str(&text).ok()
    }

    fn set_config_value(&self, key: &str, value: Value) -> Result<(), AbiError> {
        self.set_config(key, &value.to_string())
    }

    fn simplify_invoke_json(&self, op: Value) -> Result<RequestId, AbiError> {
        let op = if op.is_object() {
            op.to_string()
        } else {
            "{}".to_owned()
        };
        self.simplify_invoke(&op)
    }

    fn simplify_invoke_result_json(&self, op: Value) -> Result<Value, AbiError> {
        let op = if op.is_object() {
            op.to_string()
        } else {
            "{}".to_owned()
        };
        let text = self.simplify_invoke_result(&op);
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|_| AbiError::BadString)
    }
}
