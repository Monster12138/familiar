import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

let isVisible = false;

async function updateStats() {
    try {
        const stats = await invoke("get_system_stats");
        
        // CPU
        const cpuPercent = Math.min(100, Math.max(0, stats.cpu_usage)).toFixed(1);
        document.getElementById('bar-cpu').style.width = `${cpuPercent}%`;
        document.getElementById('val-cpu').innerText = `${Math.round(cpuPercent)}%`;

        // RAM
        const ramPercent = stats.memory_total > 0 ? ((stats.memory_used / stats.memory_total) * 100).toFixed(1) : 0;
        document.getElementById('bar-ram').style.width = `${ramPercent}%`;
        document.getElementById('val-ram').innerText = `${Math.round(ramPercent)}%`;

        // Disk
        const diskPercent = stats.disk_total > 0 ? ((stats.disk_used / stats.disk_total) * 100).toFixed(1) : 0;
        document.getElementById('bar-disk').style.width = `${diskPercent}%`;
        document.getElementById('val-disk').innerText = `${Math.round(diskPercent)}%`;

        if (!isVisible) {
            isVisible = true;
        }
    } catch (e) {
        console.error("Failed to fetch stats:", e);
    }
}

window.addEventListener("DOMContentLoaded", () => {
    // Make stats drag the main window
    document.getElementById('stats-container').addEventListener('mousedown', (e) => {
        if (e.button === 0) {
            invoke('drag_main_window').catch(console.error);
        }
    });

    // Poll every 2 seconds
    updateStats();
    setInterval(updateStats, 2000);
});
