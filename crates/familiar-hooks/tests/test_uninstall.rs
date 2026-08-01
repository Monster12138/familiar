use familiar_hooks::antigravity::AntigravityHook;
use familiar_hooks::hook_trait::AgentHook;

#[test]
fn test_it() {
    let hook = AntigravityHook::new();
    println!("Is injected: {}", hook.is_injected());
    hook.uninstall().unwrap();
    println!("Is injected after: {}", hook.is_injected());
}
