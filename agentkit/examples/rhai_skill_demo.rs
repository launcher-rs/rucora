//! Rhai 脚本技能示例（简化版）
//!
//! 演示如何使用 Rhai 脚本定义技能。
//! 注意：需要启用 `rhai-skills` feature
//!
//! 运行方式：
//! - `cargo run -p agentkit --features rhai-skills --example rhai_skill_demo`

#[cfg(feature = "rhai-skills")]
fn main() {
    println!("=== Rhai 脚本技能示例 ===\n");
    println!("✓ Rhai 功能已启用\n");
    
    // 演示 Rhai 脚本的基本使用
    let engine = rhai::Engine::new();
    
    // 注册自定义函数
    engine.register_fn("greet", |name: &str| -> String {
        format!("你好，{}！", name)
    });
    
    // 执行脚本
    let script = r#"
        let name = "Rhai";
        greet(name)
    "#;
    
    match engine.eval::<String>(script) {
        Ok(result) => {
            println!("✓ 脚本执行成功");
            println!("  结果：{}\n", result);
        }
        Err(e) => {
            println!("❌ 脚本执行失败：{}\n", e);
        }
    }
    
    println!("=== 示例完成 ===");
    println!("\n提示：完整的 Rhai 技能示例需要配置 skills 目录和工具链");
}

#[cfg(not(feature = "rhai-skills"))]
fn main() {
    println!("=== Rhai 脚本技能示例 ===\n");
    println!("⚠ Rhai 功能未启用\n");
    println!("启用方法:");
    println!("  cargo run -p agentkit --features rhai-skills --example rhai_skill_demo\n");
    println!("=== 示例完成 ===");
}
