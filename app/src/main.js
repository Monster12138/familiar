import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";

import { PixelSpriteRenderer } from "./pet/PixelSpriteRenderer.js";
import { applyTranslations, t } from "./i18n.js";

let renderer = null;
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
    
    // Dragging is now handled specifically by PixelSpriteRenderer, BubbleOverlay, and stats.js

    setupContextMenu();

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

        // Clear any pending idle fallback
        if (window.idleFallbackTimer) {
            clearTimeout(window.idleFallbackTimer);
            window.idleFallbackTimer = null;
        }

        // Map mood to animation
        switch (state.mood) {
            case "Thinking":
            case "Busy":
                renderer.playAnimation("working");
                break;
            case "Happy":
            case "Celebrating":
                renderer.playAnimation("happy");
                // Automatically revert to idle after celebration
                window.idleFallbackTimer = setTimeout(() => {
                    renderer.playAnimation("idle");
                }, 4000);
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

    // Unified window architecture - no need to sync aux windows
}

function updateWindowSize() {
    const container = document.getElementById('unified-container');
    if (!container) return;
    const rect = container.getBoundingClientRect();
    const width = Math.ceil(rect.width);
    const height = Math.ceil(rect.height);
    
    try {
        const appWindow = getCurrentWebviewWindow();
        appWindow.setSize(new LogicalSize(width, height)).catch(console.error);
    } catch (e) {
        console.error("Failed to resize window dynamically:", e);
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
    
    if (petConf.opacity !== undefined) {
        document.body.style.opacity = petConf.opacity;
    }
    
    if (petConf.scale !== undefined && renderer) {
        window.petScale = petConf.scale;
        
        const container = document.getElementById('unified-container');
        if (container) {
            container.style.transform = `scale(${window.petScale})`;
            
            // Force an immediate update because CSS transform doesn't trigger ResizeObserver
            updateWindowSize();
            
            // Dynamically fit the Tauri window to the exact bounding box of the container
            if (!window.containerObserver) {
                window.containerObserver = new ResizeObserver(() => {
                    updateWindowSize();
                });
                window.containerObserver.observe(container);
            }
        }
    }
}

window.addEventListener("DOMContentLoaded", () => {
    init();
});
