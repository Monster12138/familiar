import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyTranslations, t } from "./i18n.js";
import { BubbleOverlay } from "./pet/BubbleOverlay.js";

let bubbleOverlay = null;
let currentLang = 'en-US';




let currentConfig = null;

async function applyConfigToWindow(config) {
    if (!config) return;
    currentConfig = config;

    if (config.general && config.general.language) {
        currentLang = config.general.language;
        applyTranslations(currentLang);
    }

    if (!config.renderer || !config.renderer['desktop-pet']) return;
    
    // Global scale is handled by unified-container in main.js
    // Bubble scale is no longer supported as an independent config
}

async function init() {
    bubbleOverlay = new BubbleOverlay();

    // Listen for state changes from Rust Backend
    listen("state_changed", async (event) => {
        const state = event.payload;
        if (!state) return;

        // Show bubbles for active agents (excluding hidden sessions)
        const hiddenSessions = currentConfig?.sessions?.hidden_sessions || [];
        const activeAgents = state.agents.filter(a =>
            !hiddenSessions.includes(a.id) &&
            ["Thinking", "Working", "Completed", "WaitingInput", "Idle"].includes(a.status)
        );
        bubbleOverlay.render(activeAgents, currentLang, t);
    });

    // Listen for config changes from Rust Backend
    listen("config_changed", (event) => {
        applyConfigToWindow(event.payload);
    });

    try {
        const config = await invoke("get_config");
        applyConfigToWindow(config);
    } catch (e) {
        console.error("Failed to load initial config", e);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    init();
});
