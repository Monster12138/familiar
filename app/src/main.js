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

async function init() {
    const container = document.getElementById("pet-container");
    
    // Enable dragging on the whole window when clicking the pet area
    const appWindow = getCurrentWebviewWindow();
    document.body.addEventListener('mousedown', (e) => {
        if (e.buttons === 1) { // Left click
            appWindow.startDragging();
        }
    });

    // Disable default right-click context menu (e.g. Inspect Element)
    document.addEventListener('contextmenu', e => e.preventDefault());

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
