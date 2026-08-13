import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export class BubbleOverlay {
    constructor() {
        this.container = document.getElementById('bubble-container');
        this.container.style.pointerEvents = 'none';
        this.bubbles = new Map(); // sessionId -> { element, timeoutId }
    }

    getSpinnerIcon() {
        return `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" style="animation: spin 1s linear infinite;"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path><style>@keyframes spin { 100% { transform: rotate(360deg); } }</style></svg>`;
    }

    getCheckmarkIcon() {
        return `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
    }

    getIdleIcon() {
        return `<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" style="color: #9ca3af;"><circle cx="5" cy="12" r="2.5"></circle><circle cx="12" cy="12" r="2.5"></circle><circle cx="19" cy="12" r="2.5"></circle></svg>`;
    }

    getSourceBadge(source) {
        const srcStr = typeof source === 'string' ? source : (source?.Custom || 'Agent');
        if (srcStr === 'Codex') {
            return `<span style="display:inline-flex; align-items:center; background:#10a37f; color:#ffffff; font-size:9px; font-weight:700; padding:1px 4px; border-radius:4px; line-height:1.2;">Codex</span>`;
        } else if (srcStr === 'ClaudeCode' || srcStr === 'Claude') {
            return `<span style="display:inline-flex; align-items:center; background:#d97706; color:#ffffff; font-size:9px; font-weight:700; padding:1px 4px; border-radius:4px; line-height:1.2;">Claude</span>`;
        } else if (srcStr === 'Antigravity' || srcStr === 'Agy') {
            return `<span style="display:inline-flex; align-items:center; background:#4f46e5; color:#ffffff; font-size:9px; font-weight:700; padding:1px 4px; border-radius:4px; line-height:1.2;">AGY</span>`;
        } else if (srcStr === 'Qoder') {
            return `<span style="display:inline-flex; align-items:center; background:#0284c7; color:#ffffff; font-size:9px; font-weight:700; padding:1px 4px; border-radius:4px; line-height:1.2;">Qoder</span>`;
        }
        return `<span style="display:inline-flex; align-items:center; background:#6b7280; color:#ffffff; font-size:9px; font-weight:700; padding:1px 4px; border-radius:4px; line-height:1.2;">${srcStr}</span>`;
    }

    createBubbleElement() {
        const bubble = document.createElement('div');
        bubble.style.padding = '8px 12px';
        bubble.style.background = 'rgba(255, 255, 255, 0.95)';
        bubble.style.border = '2px solid #333';
        bubble.style.borderRadius = '16px 16px 16px 16px';
        bubble.style.fontFamily = 'system-ui, sans-serif';
        bubble.style.fontSize = '12px';
        bubble.style.fontWeight = '500';
        bubble.style.color = '#333';
        bubble.style.opacity = '0';
        bubble.style.transition = 'opacity 0.3s ease-in-out';
        bubble.style.pointerEvents = 'auto';
        bubble.style.width = '100%';
        bubble.style.boxSizing = 'border-box';
        bubble.style.boxShadow = '2px 2px 0px rgba(0,0,0,0.1)';
        bubble.style.cursor = 'grab';
        
        bubble.addEventListener('mousedown', (e) => {
            if (e.button === 0) {
                import("@tauri-apps/api/core").then(({ invoke }) => {
                    invoke('drag_main_window').catch(console.error);
                });
            }
        });

        const innerContainer = document.createElement('div');
        innerContainer.style.display = 'flex';
        innerContainer.style.flexDirection = 'column';
        innerContainer.style.gap = '4px';
        bubble.appendChild(innerContainer);

        const headerEl = document.createElement('div');
        headerEl.style.display = 'flex';
        headerEl.style.alignItems = 'center';
        headerEl.style.gap = '4px';
        headerEl.style.width = '100%';
        innerContainer.appendChild(headerEl);

        const sourceBadgeEl = document.createElement('div');
        sourceBadgeEl.style.flexShrink = '0';
        headerEl.appendChild(sourceBadgeEl);

        const userInstructionEl = document.createElement('div');
        userInstructionEl.style.color = '#666';
        userInstructionEl.style.fontSize = '10px';
        userInstructionEl.style.fontStyle = 'italic';
        userInstructionEl.style.whiteSpace = 'nowrap';
        userInstructionEl.style.overflow = 'hidden';
        userInstructionEl.style.textOverflow = 'ellipsis';
        userInstructionEl.style.width = '100%';
        userInstructionEl.style.display = 'block';
        headerEl.appendChild(userInstructionEl);

        const activityContainer = document.createElement('div');
        activityContainer.style.display = 'flex';
        activityContainer.style.alignItems = 'center';
        activityContainer.style.gap = '6px';
        activityContainer.style.maxWidth = '100%';
        activityContainer.style.overflow = 'hidden';
        innerContainer.appendChild(activityContainer);

        const iconEl = document.createElement('div');
        iconEl.style.display = 'flex';
        iconEl.style.alignItems = 'center';
        iconEl.style.justifyContent = 'center';
        iconEl.style.flexShrink = '0';
        activityContainer.appendChild(iconEl);

        const activityEl = document.createElement('div');
        activityEl.style.color = '#333';
        activityEl.style.fontSize = '12px';
        activityEl.style.fontWeight = '600';
        activityEl.style.whiteSpace = 'nowrap';
        activityEl.style.overflow = 'hidden';
        activityEl.style.textOverflow = 'ellipsis';
        activityEl.style.width = '100%';
        activityEl.style.display = 'block';
        activityContainer.appendChild(activityEl);

        return { bubble, sourceBadgeEl, userInstructionEl, iconEl, activityEl };
    }

    render(agents, currentLang, translateFn) {
        // Only show up to 3 most recently active agents to prevent covering the whole screen
        const activeAgents = agents.slice(-3);
        const activeIds = new Set(activeAgents.map(a => a.id));

        // Remove stale bubbles
        for (const [id, data] of this.bubbles.entries()) {
            if (!activeIds.has(id)) {
                data.element.bubble.style.opacity = '0';
                setTimeout(() => {
                    if (this.container.contains(data.element.bubble)) {
                        this.container.removeChild(data.element.bubble);
                    }
                }, 300);
                if (data.timeoutId) clearTimeout(data.timeoutId);
                this.bubbles.delete(id);
            }
        }

        // Add or update bubbles
        activeAgents.forEach((agent, index) => {
            let data = this.bubbles.get(agent.id);
            if (!data) {
                data = { element: this.createBubbleElement(), timeoutId: null };
                this.container.appendChild(data.element.bubble);
                this.bubbles.set(agent.id, data);
                // Trigger reflow for opacity transition
                void data.element.bubble.offsetWidth; 
            }

            // Adjust bottom-right rounding based on position to simulate a tail for the bottom-most bubble
            data.element.bubble.style.borderRadius = index === 0 ? '16px 16px 0px 16px' : '16px';

            const ui = data.element;
            ui.sourceBadgeEl.innerHTML = this.getSourceBadge(agent.source);
            ui.userInstructionEl.textContent = (agent.user_instruction || translateFn("status_waiting", currentLang)).replace(/\r?\n/g, ' ');
            ui.activityEl.textContent = (agent.current_activity || "").replace(/\r?\n/g, ' ');

            let statusType = 'working';
            if (agent.status === 'Completed') statusType = 'completed';
            else if (agent.status === 'Pending' || agent.status === 'Idle') statusType = 'idle';

            if (statusType === 'completed') {
                ui.iconEl.innerHTML = this.getCheckmarkIcon();
            } else if (statusType === 'working') {
                ui.iconEl.innerHTML = this.getSpinnerIcon();
            } else {
                ui.iconEl.innerHTML = this.getIdleIcon();
            }

            ui.bubble.style.opacity = '1';
        });
        
        // Unified window handles its own size, no need to resize here
    }
}
