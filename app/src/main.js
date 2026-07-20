import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import { PixelSpriteRenderer } from "./pet/PixelSpriteRenderer.js";
import { BubbleOverlay } from "./pet/BubbleOverlay.js";

let renderer = null;
let bubbleOverlay = null;

async function fetchManifest(spriteName) {
    const res = await fetch(`/sprites/${spriteName}/manifest.json`);
    return await res.json();
}

function setupContextMenu() {
    const contextMenu = document.getElementById("context-menu");
    const settingsModalBackdrop = document.getElementById("settings-modal-backdrop");
    const settingsModal = document.getElementById("settings-modal");
    
    const menuSettings = document.getElementById("menu-settings");
    const btnCancelSettings = document.getElementById("btn-cancel-settings");
    const btnSaveSettings = document.getElementById("btn-save-settings");

    // Inputs
    const hookAntigravity = document.getElementById("setting-hook-antigravity");
    const hookClaude = document.getElementById("setting-hook-claude");
    const ipcUds = document.getElementById("setting-ipc-uds");
    const ipcTcp = document.getElementById("setting-ipc-tcp");

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
            // Populate these values on load
            const config = await invoke("get_config");
            if (config) {
                // Try to handle nested or flat structure gracefully
                if (config.hooks) {
                    hookAntigravity.checked = !!config.hooks.antigravity;
                    hookClaude.checked = !!config.hooks.claude_code;
                } else {
                    hookAntigravity.checked = !!config.hook_antigravity;
                    hookClaude.checked = !!config.hook_claude_code;
                }
                
                if (config.ipc) {
                    ipcUds.value = config.ipc.uds_path || "/tmp/familiar.sock";
                    ipcTcp.value = config.ipc.tcp_port || 9528;
                } else {
                    ipcUds.value = config.ipc_uds_path || "/tmp/familiar.sock";
                    ipcTcp.value = config.ipc_tcp_port || 9528;
                }
            }
        } catch (e) {
            console.error("Failed to load config:", e);
        }

        settingsModalBackdrop.style.display = "flex";
    });

    // Close Settings Modal
    const closeModal = () => {
        settingsModalBackdrop.style.display = "none";
    };

    btnCancelSettings.addEventListener("click", closeModal);

    // Close on click outside or Escape
    settingsModalBackdrop.addEventListener("click", (e) => {
        if (e.target === settingsModalBackdrop) {
            closeModal();
        }
    });

    document.addEventListener("keydown", (e) => {
        if (e.key === "Escape" && settingsModalBackdrop.style.display === "flex") {
            closeModal();
        }
    });

    // Save Settings
    btnSaveSettings.addEventListener("click", async () => {
        const modifiedConfig = {
            hooks: {
                antigravity: hookAntigravity.checked,
                claude_code: hookClaude.checked
            },
            ipc: {
                uds_path: ipcUds.value,
                tcp_port: parseInt(ipcTcp.value, 10) || 9528
            }
        };

        try {
            await invoke("save_config", { config: modifiedConfig });
            closeModal();
        } catch (e) {
            console.error("Failed to save config:", e);
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
                activeAgent.user_instruction || "Waiting for task...",
                activeAgent.current_activity || "",
                isCompleted,
                duration
            );
        }
    });
}

window.addEventListener("DOMContentLoaded", () => {
    init();
});
