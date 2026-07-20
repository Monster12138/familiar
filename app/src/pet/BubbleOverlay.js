import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export class BubbleOverlay {
    constructor() {
        this.bubble = document.createElement('div');
        this.bubble.style.position = 'fixed';
        // Anchor to the bottom, perfectly tracking the cat's scaled sprite height
        this.bubble.style.bottom = 'calc(100vw / 1.777 - 10px)';
        this.bubble.style.left = '10px';
        this.bubble.style.padding = '8px 12px'; // slightly smaller padding to save space
        this.bubble.style.background = 'rgba(255, 255, 255, 0.95)';
        this.bubble.style.border = '2px solid #333';
        this.bubble.style.borderRadius = '16px 16px 0px 16px'; // Chat bubble tail points to bottom right
        this.bubble.style.fontFamily = 'system-ui, sans-serif';
        this.bubble.style.fontSize = '12px'; // slightly smaller font to fit 320px width better
        this.bubble.style.fontWeight = '500';
        this.bubble.style.color = '#333';
        this.bubble.style.opacity = '0';
        this.bubble.style.transition = 'opacity 0.3s ease-in-out';
        this.bubble.style.pointerEvents = 'auto';
        this.bubble.style.whiteSpace = 'normal';
        this.bubble.style.width = 'fit-content';
        this.bubble.style.maxWidth = '280px'; // Fits safely inside 320px window width
        this.bubble.style.overflow = 'hidden';
        this.bubble.style.display = '-webkit-box';
        this.bubble.style.webkitBoxOrient = 'vertical';
        this.bubble.style.webkitLineClamp = '2'; // Limit to 2 lines
        this.bubble.style.wordBreak = 'break-word';
        this.bubble.style.boxShadow = '2px 2px 0px rgba(0,0,0,0.1)';
        this.bubble.style.zIndex = '1000';
        this.bubble.style.cursor = 'grab';
        
        document.body.appendChild(this.bubble);
        this.timeoutId = null;

        this.bubble.addEventListener('mousedown', (e) => {
            if (e.button === 0) {
                getCurrentWebviewWindow().startDragging();
            }
        });

        // Create inner elements
        this.innerContainer = document.createElement('div');
        this.innerContainer.style.display = 'flex';
        this.innerContainer.style.flexDirection = 'column';
        this.innerContainer.style.gap = '4px';
        this.innerContainer.style.width = '100%';
        this.bubble.appendChild(this.innerContainer);

        this.userInstructionEl = document.createElement('div');
        this.userInstructionEl.style.color = '#666';
        this.userInstructionEl.style.fontSize = '10px';
        this.userInstructionEl.style.fontStyle = 'italic';
        this.userInstructionEl.style.display = '-webkit-box';
        this.userInstructionEl.style.webkitBoxOrient = 'vertical';
        this.userInstructionEl.style.webkitLineClamp = '2';
        this.userInstructionEl.style.overflow = 'hidden';
        this.userInstructionEl.style.wordBreak = 'break-word';
        this.innerContainer.appendChild(this.userInstructionEl);

        this.activityContainer = document.createElement('div');
        this.activityContainer.style.display = 'flex';
        this.activityContainer.style.alignItems = 'center';
        this.activityContainer.style.gap = '6px';
        this.innerContainer.appendChild(this.activityContainer);

        this.iconEl = document.createElement('div');
        this.iconEl.style.display = 'flex';
        this.iconEl.style.alignItems = 'center';
        this.iconEl.style.justifyContent = 'center';
        this.activityContainer.appendChild(this.iconEl);

        this.activityEl = document.createElement('div');
        this.activityEl.style.color = '#333';
        this.activityEl.style.fontSize = '12px';
        this.activityEl.style.fontWeight = '600';
        this.activityEl.style.display = '-webkit-box';
        this.activityEl.style.webkitBoxOrient = 'vertical';
        this.activityEl.style.webkitLineClamp = '2';
        this.activityEl.style.overflow = 'hidden';
        this.activityEl.style.wordBreak = 'break-word';
        this.activityContainer.appendChild(this.activityEl);

        window.addEventListener('pet-bubble', (e) => {
            this.show(e.detail.userInstruction, e.detail.currentActivity, e.detail.isCompleted, e.detail.duration);
        });
    }

    getSpinnerIcon() {
        return `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" style="animation: spin 1s linear infinite;"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path><style>@keyframes spin { 100% { transform: rotate(360deg); } }</style></svg>`;
    }

    getCheckmarkIcon() {
        return `<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#22c55e" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
    }

    show(userInstruction, currentActivity, isCompleted, duration = 3000) {
        this.userInstructionEl.innerText = userInstruction;
        this.activityEl.innerText = currentActivity;
        
        if (isCompleted) {
            this.iconEl.innerHTML = this.getCheckmarkIcon();
        } else {
            this.iconEl.innerHTML = this.getSpinnerIcon();
        }

        this.bubble.style.opacity = '1';
        
        if (this.timeoutId) {
            clearTimeout(this.timeoutId);
        }
        
        this.timeoutId = setTimeout(() => {
            this.bubble.style.opacity = '0';
        }, duration);
    }
    
    setScale(scale) {
        this.bubble.style.transformOrigin = 'bottom left';
        this.bubble.style.transform = `scale(${scale})`;
    }
}
