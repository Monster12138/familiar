import { applyTranslations, t } from './i18n.js';

const { invoke } = window.__TAURI__.core;
const { getCurrentWebviewWindow } = window.__TAURI__.webviewWindow;

let lang = 'zh-CN';

// Mark onboarding complete so the welcome page does not reappear.
async function completeOnboarding() {
    try {
        await invoke('complete_onboarding');
    } catch (e) {
        console.error('Failed to complete onboarding', e);
    }
}

// The first page is the introduction; the next page is the dedicated hook
// manager window. Completing onboarding here keeps the intro from coming
// back, then open the manager and close this window.
async function nextToHooks() {
    await completeOnboarding();
    try {
        await invoke('open_hook_manager_window');
    } catch (e) {
        console.error('Failed to open hook manager', e);
    }
    const win = getCurrentWebviewWindow();
    if (win) win.close();
}

// Skip setup: just mark onboarding complete and close.
async function skipOnboarding() {
    await completeOnboarding();
    const win = getCurrentWebviewWindow();
    if (win) win.close();
}

document.addEventListener('DOMContentLoaded', async () => {
    try {
        const config = await invoke('get_config');
        if (config && config.general && config.general.language) {
            lang = config.general.language;
        }
    } catch (e) {
        console.error('Failed to load config', e);
    }
    applyTranslations(lang);

    const nextBtn = document.getElementById('btn-onboard-next');
    const skipBtn = document.getElementById('btn-onboard-skip');
    if (nextBtn) nextBtn.addEventListener('click', nextToHooks);
    if (skipBtn) skipBtn.addEventListener('click', skipOnboarding);
});
