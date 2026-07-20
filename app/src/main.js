import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";

import { PixelSpriteRenderer } from "./pet/PixelSpriteRenderer.js";
import { BubbleOverlay } from "./pet/BubbleOverlay.js";
import { applyTranslations, t } from "./i18n.js";

let renderer = null;
let bubbleOverlay = null;
let currentLang = 'en-US';

async function fetchManifest(spriteName) {
    const res = await fetch(`/sprites/${spriteName}/manifest.json`);
    return await res.json();
}

function setupContextMenu() {
    const contextMenu = document.getElementById("context-menu");
    const menuSettings = document.getElementById("menu-settings");

    // Custom Context Menu
    document.addEventListener('contextmenu', e => {
        e.preventDefault();
        // Position menu at mouse coordinates
        contextMenu.style.display = "block";
        
        // Ensure menu doesn't go off screen
        const menuWidth = contextMenu.offsetWidth || 150;
        const menuHeight = contextMenu.offsetHeight || 40;
        
        let x = e.clientX;
        let y = e.clientY;
        
        if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth;
        if (y + menuHeight > window.innerHeight) y = window.innerHeight - menuHeight;
        
        contextMenu.style.left = `${x}px`;
        contextMenu.style.top = `${y}px`;
    });

    document.addEventListener('click', e => {
        if (e.target !== contextMenu && !contextMenu.contains(e.target)) {
            contextMenu.style.display = "none";
        }
    });

    // Settings Modal Open
    menuSettings.addEventListener("click", async () => {
        contextMenu.style.display = "none";
        try {
            await invoke("open_settings_window");
        } catch (e) {
            console.error("Failed to open settings window:", e);
        }
    });
}

async function init() {
    const container = document.getElementById("pet-container");
    
    // Enable dragging on the whole window when clicking the pet area
    const appWindow = getCurrentWebviewWindow();
    document.body.addEventListener('mousedown', (e) => {
        if (e.buttons === 1) { // Left click
            appWindow.startDragging();
        }
    });

    setupContextMenu();

    bubbleOverlay = new BubbleOverlay();
    renderer = new PixelSpriteRenderer();
    renderer.init(container);

    try {
        const manifest = await fetchManifest("pixel-cat");
        await renderer.loadSpritePack(manifest);
        renderer.playAnimation("idle");
    } catch (e) {
        console.error("Failed to load sprite pack", e);
    }

    // Listen for state changes from Rust Backend
    await listen("state_changed", (event) => {
        const state = event.payload;
        if (!state) return;

        // Map mood to animation
        switch (state.mood) {
            case "Busy":
                renderer.playAnimation("working");
                break;
            case "Thinking":
                renderer.playAnimation("thinking");
                break;
            case "Happy":
            case "Celebrating":
                renderer.playAnimation("happy");
                break;
            case "Alarmed":
                renderer.playAnimation("alarmed");
                break;
            case "Sleepy":
                renderer.playAnimation("sleeping");
                break;
            default:
                renderer.playAnimation("idle");
        }

        // Show a bubble for the latest active agent activity
        const activeAgent = state.agents.find(a => ["Thinking", "Working", "Completed", "WaitingInput"].includes(a.status));
        if (activeAgent && (activeAgent.current_activity || activeAgent.user_instruction)) {
            const isCompleted = ["Completed", "WaitingInput"].includes(activeAgent.status);
            const duration = isCompleted ? 3000 : 2000; // Keep checkmark visible a bit longer
            renderer.showBubble(
                activeAgent.user_instruction || t("status_waiting", currentLang),
                activeAgent.current_activity || "",
                isCompleted,
                duration
            );
        }
    });

    // Listen for config changes from Rust Backend
    await listen("config_changed", (event) => {
        applyConfigToWindow(event.payload);
    });

    try {
        const config = await invoke("get_config");
        applyConfigToWindow(config);
    } catch (e) {
        console.error("Failed to load initial config", e);
    }
}

async function applyConfigToWindow(config) {
    if (!config) return;

    if (config.general && config.general.language) {
        currentLang = config.general.language;
        applyTranslations(currentLang);
    }

    if (!config.renderer || !config.renderer['desktop-pet']) return;
    
    const petConf = config.renderer['desktop-pet'];
    const appWindow = getCurrentWebviewWindow();
    
    if (petConf.opacity !== undefined) {
        document.body.style.opacity = petConf.opacity;
    }
    
    if (petConf.bubble_scale !== undefined && bubbleOverlay) {
        bubbleOverlay.setScale(petConf.bubble_scale);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    init();
});
