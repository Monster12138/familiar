use familiar_hooks::hook_trait::AgentHook;
use familiar_hooks::antigravity::AntigravityHook;

#[test]
fn test_it() {
    let hook = AntigravityHook::new();
    println!("Is injected: {}", hook.is_injected());
    hook.uninstall().unwrap();
    println!("Is injected after: {}", hook.is_injected());
}
