import { applyTranslations, t } from './i18n.js';
import { mountHookPanel } from './hook-panel.js';

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

// Step 2: swap the introduction for the shared hook management panel
// mounted in the same window.
async function showHooks() {
    const intro = document.getElementById('onboard-intro');
    const hookView = document.getElementById('onboard-hook-view');
    if (intro) intro.style.display = 'none';
    if (hookView) hookView.style.display = 'block';
    await mountHookPanel(document.getElementById('onboard-hook-panel'));
}

// Finish setup after the hooks step: complete onboarding and close.
async function finishOnboarding() {
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
    const doneBtn = document.getElementById('btn-onboard-done');
    if (nextBtn) nextBtn.addEventListener('click', showHooks);
    if (skipBtn) skipBtn.addEventListener('click', finishOnboarding);
    if (doneBtn) doneBtn.addEventListener('click', finishOnboarding);
});
