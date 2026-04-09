//! Rhai 脚本技能实现。
//!
//! 本模块提供基于 Rhai 脚本语言的技能实现，允许使用脚本描述技能逻辑。

#[cfg(feature = "rhai-skills")]
use agentkit_core::{error::SkillError, tool::ToolCategory};
#[cfg(feature = "rhai-skills")]
use async_trait::async_trait;
#[cfg(feature = "rhai-skills")]
use rhai::{Engine, Scope};
#[cfg(feature = "rhai-skills")]
use serde_json::{Value, json};
#[cfg(feature = "rhai-skills")]
use std::sync::Arc;
#[cfg(feature = "rhai-skills")]
use std::sync::{OnceLock, RwLock};
#[cfg(feature = "rhai-skills")]
use tracing::{debug, info};

#[cfg(feature = "rhai-skills")]
pub use agentkit_core::skill::Skill;

#[cfg(feature = "rhai-skills")]
static GLOBAL_RHAI_REGISTRAR: OnceLock<RwLock<Option<RhaiEngineRegistrar>>> = OnceLock::new();

#[cfg(feature = "rhai-skills")]
fn global_rhai_registrar_cell() -> &'static RwLock<Option<RhaiEngineRegistrar>> {
    GLOBAL_RHAI_REGISTRAR.get_or_init(|| RwLock::new(None))
}

/// 设置全局 Rhai 引擎注册器。
///
/// 该函数用于在应用启动时注册全局的 Rhai 引擎配置器，
/// 以便所有 Rhai 技能共享相同的宿主函数注册逻辑。
#[cfg(feature = "rhai-skills")]
pub fn set_global_rhai_engine_registrar(registrar: Option<RhaiEngineRegistrar>) {
    let cell = global_rhai_registrar_cell();
    let mut w = cell.write().expect("global rhai registrar lock poisoned");
    *w = registrar;
}

/// 获取全局 Rhai 引擎注册器。
#[cfg(feature = "rhai-skills")]
pub fn get_global_rhai_engine_registrar() -> Option<RhaiEngineRegistrar> {
    let cell = global_rhai_registrar_cell();
    let r = cell.read().expect("global rhai registrar lock poisoned");
    r.clone()
}

/// Rhai 引擎的"宿主函数注册器"。
///
/// 说明：
/// - blockcell 的 SKILL.rhai 脚本里通常会调用 `call_tool(...)`、`browse(...)` 等宿主函数。
/// - 这些函数不是 Rhai 自带的，需要宿主程序在创建 `Engine` 时注册。
/// - agentkit 在 core/skills 层不强行绑定具体工具集，因此通过该接口把注册权交给上层。
///
/// 你可以在应用启动时：
/// - 实现一个 registrar：向 `Engine` 注册你希望脚本可用的函数
/// - 再调用 `load_skills_from_dir_with_rhai(...)` 加载脚本 skills
#[cfg(feature = "rhai-skills")]
pub type RhaiEngineRegistrar = Arc<dyn Fn(&mut Engine) + Send + Sync>;

/// Rhai 脚本通过 `call_tool("xxx", args)` 调用宿主工具时所使用的回调。
///
/// 约定：该 invoker 为"同步调用"。
/// - Rhai 引擎本身是同步执行的（不支持直接 await）。
/// - 因此默认实现不会在这里做异步阻塞等待，以避免在 `current_thread` runtime 下产生 panic。
/// - 若上层需要在 Rhai 中调用 async tool，可自行提供一个封装 invoker（例如在多线程 runtime 中
///   `block_in_place + Handle::block_on`），或采用其它消息队列/事件机制。
#[cfg(feature = "rhai-skills")]
pub type RhaiToolInvoker = Arc<dyn Fn(&str, Value) -> Result<Value, String> + Send + Sync>;

/// Rhai skill 的"宿主标准库"注册器。
///
/// 目标：让 blockcell 风格的 SKILL.rhai 不需要每个项目都手写 registrar。
///
/// 当前提供：
/// - `call_tool(name, args)`：调用宿主工具
/// - `is_error(x)` / `is_map(x)`：便捷判断
/// - `arr_join(arr, delim)`：数组拼接
/// - `log_info/log_debug`：日志输出
/// - `json_parse/json_stringify`：JSON 解析/序列化
#[cfg(feature = "rhai-skills")]
pub fn rhai_stdlib_registrar(invoker: RhaiToolInvoker) -> RhaiEngineRegistrar {
    Arc::new(move |engine: &mut Engine| {
        let invoker = invoker.clone();

        engine.register_fn("is_error", |x: rhai::Dynamic| -> bool {
            if x.is::<bool>() {
                return !x.cast::<bool>();
            }
            if x.is_map() {
                let m = x.clone().cast::<rhai::Map>();
                if let Some(v) = m.get("success") {
                    if v.is::<bool>() && !v.clone().cast::<bool>() {
                        return true;
                    }
                }
                if m.contains_key("error") {
                    return true;
                }
            }
            false
        });

        engine.register_fn("is_map", |x: rhai::Dynamic| -> bool { x.is_map() });

        engine.register_fn("arr_join", |arr: rhai::Array, delim: &str| -> String {
            let mut parts: Vec<String> = Vec::with_capacity(arr.len());
            for v in arr {
                parts.push(v.to_string());
            }
            parts.join(delim)
        });

        engine.register_fn("log_info", |msg: &str| {
            info!(rhai.log = %msg, "rhai.log_info");
        });

        engine.register_fn("log_debug", |msg: &str| {
            debug!(rhai.log = %msg, "rhai.log_debug");
        });

        engine.register_fn("json_stringify", |x: rhai::Dynamic| -> String {
            let v: Value =
                rhai::serde::from_dynamic(&x).unwrap_or_else(|_| json!({"_raw": x.to_string()}));
            v.to_string()
        });

        engine.register_fn("json_parse", |s: &str| -> rhai::Dynamic {
            match serde_json::from_str::<Value>(s) {
                Ok(v) => rhai::serde::to_dynamic(v).unwrap_or_else(|_| rhai::Dynamic::from(())),
                Err(_) => rhai::Dynamic::from(()),
            }
        });

        engine.register_fn(
            "call_tool",
            move |tool_name: &str, args: rhai::Dynamic| -> rhai::Dynamic {
                let input: Value = rhai::serde::from_dynamic(&args)
                    .unwrap_or_else(|_| json!({"_raw": args.to_string()}));

                // 说明：这里的 invoker 约定为同步调用。
                // 若上层希望在 Rhai 中调用 async tool，可自行提供一个封装后的 invoker。
                let output: Result<Value, String> = (invoker)(tool_name, input);

                match output {
                    Ok(v) => {
                        rhai::serde::to_dynamic(v).unwrap_or_else(|_| rhai::Dynamic::from(false))
                    }
                    Err(e) => rhai::serde::to_dynamic(json!({"success": false, "error": e}))
                        .unwrap_or_else(|_| rhai::Dynamic::from(false)),
                }
            },
        );
    })
}

/// 基于 `SKILL.rhai` 的脚本型 skill。
///
/// 设计目标：参考 blockcell 的 skills 形态，让每个 skill 用一段 Rhai 脚本来描述"怎么做"。
///
/// 约定：脚本运行时会注入 `ctx`：
/// - `ctx.user_input`：用户原始输入文本
/// - `ctx.input`：本次 tool call 的 JSON input（serde_json::Value）
///
/// 脚本返回值：
/// - 推荐返回一个 map（例如 `#{ success: true, instruction: "..." }`）
/// - host 会尽量把返回值转成 JSON Value 作为 tool result 回传
#[cfg(feature = "rhai-skills")]
pub struct RhaiSkill {
    pub name: String,
    pub description: Option<String>,
    pub script_source: String,
    pub rhai_registrar: Option<RhaiEngineRegistrar>,
}

#[cfg(feature = "rhai-skills")]
impl RhaiSkill {
    /// 创建新的 Rhai 技能实例。
    pub fn new(
        name: String,
        description: Option<String>,
        script_source: String,
        rhai_registrar: Option<RhaiEngineRegistrar>,
    ) -> Self {
        Self {
            name,
            description,
            script_source,
            rhai_registrar,
        }
    }
}

#[cfg(feature = "rhai-skills")]
#[async_trait]
impl Skill for RhaiSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn categories(&self) -> &'static [ToolCategory] {
        &[ToolCategory::Basic]
    }

    fn input_schema(&self) -> Value {
        // 这里不强制 schema：不同脚本的入参结构可能不同。
        // 统一给一个 object，以便 LLM 可以自由传参。
        json!({
            "type": "object",
            "description": "SKILL.rhai 脚本输入（由脚本自行解析 ctx.input / ctx.user_input）"
        })
    }

    async fn run_value(&self, input: Value) -> Result<Value, SkillError> {
        debug!(skill.name = %self.name, skill.kind = "rhai", "rhai_skill.start");

        // 说明：Rhai 引擎本身是同步执行的。
        // 这里先用最小实现：直接在当前线程运行脚本。
        // 如果后续脚本变重，可以考虑 spawn_blocking。
        let mut engine = Engine::new();
        if let Some(reg) = &self.rhai_registrar {
            reg(&mut engine);
        }

        // 注入 ctx
        // - ctx.user_input：如果外部没有传入，就用空字符串
        // - ctx.input：本次调用参数
        let mut ctx_map = rhai::Map::new();
        let user_input = input
            .get("user_input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        ctx_map.insert("user_input".into(), user_input.into());

        let dyn_input = rhai::serde::to_dynamic(input.clone())
            .map_err(|e| SkillError::Message(format!("rhai: input 转 dynamic 失败：{}", e)))?;
        ctx_map.insert("input".into(), dyn_input);

        let ctx_dynamic: rhai::Dynamic = ctx_map.into();

        let mut scope = Scope::new();
        scope.push_dynamic("ctx", ctx_dynamic);

        let result = engine
            .eval_with_scope::<rhai::Dynamic>(&mut scope, &self.script_source)
            .map_err(|e| SkillError::Message(format!("rhai 脚本执行失败：{}", e)))?;

        // 返回值尽量转成 JSON
        let out: Value = rhai::serde::from_dynamic(&result)
            .unwrap_or_else(|_| json!({"success": true, "result": result.to_string()}));
        Ok(out)
    }
}


