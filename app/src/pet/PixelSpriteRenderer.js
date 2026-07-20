import { SpriteRenderer } from './SpriteRenderer.js';
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export class PixelSpriteRenderer extends SpriteRenderer {
    constructor() {
        super();
        this.canvas = null;
        this.ctx = null;
        this.manifest = null;
        this.spriteImage = null;
        
        this.currentAnim = null;
        this.currentFrameIndex = 0;
        this.lastFrameTime = 0;
        this.animationId = null;
        
        this.frameWidth = 0;
        this.frameHeight = 0;
    }

    init(container) {
        this.canvas = document.createElement('canvas');
        this.canvas.width = window.innerWidth;
        this.canvas.height = window.innerHeight;
        this.canvas.style.position = 'absolute';
        this.canvas.style.top = '0';
        this.canvas.style.left = '0';
        this.canvas.style.width = '100%';
        this.canvas.style.height = '100%';
        this.canvas.style.pointerEvents = 'none'; // allow clicks to pass through transparent pixels
        
        this.ctx = this.canvas.getContext('2d');
        // Ensure crisp pixels
        this.ctx.imageSmoothingEnabled = false;

        // Create a precise hitbox for dragging so the transparent areas don't block clicks
        this.dragHitbox = document.createElement('div');
        this.dragHitbox.style.position = 'absolute';
        this.dragHitbox.style.width = '120px';
        this.dragHitbox.style.height = '140px';
        this.dragHitbox.style.left = '50%';
        this.dragHitbox.style.bottom = '10px';
        this.dragHitbox.style.transform = 'translateX(-50%)';
        this.dragHitbox.style.cursor = 'grab';
        this.dragHitbox.style.pointerEvents = 'auto';

        // Manually trigger Tauri window drag when clicking on the hitbox
        this.dragHitbox.addEventListener('mousedown', (e) => {
            if (e.button === 0) { // Left click only
                getCurrentWebviewWindow().startDragging();
            }
        });
        
        container.appendChild(this.canvas);
        container.appendChild(this.dragHitbox);
        
        window.addEventListener('resize', () => {
            this.canvas.width = window.innerWidth;
            this.canvas.height = window.innerHeight;
            this.ctx.imageSmoothingEnabled = false;
        });
    }

    async loadSpritePack(manifest) {
        this.manifest = manifest;
        return new Promise((resolve, reject) => {
            this.spriteImage = new Image();
            this.spriteImage.onload = () => {
                // Calculate frame dimensions based on grid
                const [cols, rows] = this.manifest.grid;
                this.frameWidth = this.spriteImage.width / cols;
                this.frameHeight = this.spriteImage.height / rows;
                
                // If it's the generated fake-transparent image, we could key it out here
                this._removeBackground();
                resolve();
            };
            this.spriteImage.onerror = reject;
            // The image is expected to be served statically
            this.spriteImage.src = `/sprites/${manifest.name}/${manifest.sprite_sheet}`;
        });
    }
    
    _removeBackground() {
        const tempCanvas = document.createElement('canvas');
        tempCanvas.width = this.spriteImage.width;
        tempCanvas.height = this.spriteImage.height;
        const tCtx = tempCanvas.getContext('2d');
        tCtx.drawImage(this.spriteImage, 0, 0);
        
        const imgData = tCtx.getImageData(0, 0, tempCanvas.width, tempCanvas.height);
        const data = imgData.data;
        for (let i = 0; i < data.length; i += 4) {
            const r = data[i], g = data[i+1], b = data[i+2];
            // Green screen chroma keying
            if (g > 150 && r < 100 && b < 100) {
                data[i+3] = 0; // Transparent
            } else {
                // Keep the cat pixels as they are (do not force to black, to preserve white eyes and grays)
                data[i+3] = 255;
            }
        }
        tCtx.putImageData(imgData, 0, 0);
        
        const newImg = new Image();
        newImg.src = tempCanvas.toDataURL();
        this.spriteImage = newImg;
    }

    playAnimation(name) {
        if (!this.manifest.animations[name]) {
            console.warn(`Animation ${name} not found, falling back to idle`);
            name = 'idle';
        }
        
        if (this.currentAnim === name) return;
        
        this.currentAnim = name;
        this.currentFrameIndex = 0;
        this.lastFrameTime = performance.now();
        
        if (!this.animationId) {
            this.animationId = requestAnimationFrame((t) => this._renderLoop(t));
        }
    }

    _renderLoop(timestamp) {
        this.animationId = requestAnimationFrame((t) => this._renderLoop(t));
        
        if (!this.currentAnim || !this.manifest) return;
        
        const animData = this.manifest.animations[this.currentAnim];
        const frameDuration = 1000 / animData.fps;
        
        if (timestamp - this.lastFrameTime >= frameDuration) {
            this.currentFrameIndex++;
            if (this.currentFrameIndex >= animData.frames.length) {
                if (animData.loop) {
                    this.currentFrameIndex = 0;
                } else {
                    this.currentFrameIndex = animData.frames.length - 1;
                }
            }
            this.lastFrameTime = timestamp;
            this._drawFrame();
        }
    }

    _drawFrame() {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        
        const animData = this.manifest.animations[this.currentAnim];
        const frameId = animData.frames[this.currentFrameIndex];
        
        const [cols] = this.manifest.grid;
        const col = frameId % cols;
        const row = Math.floor(frameId / cols);
        
        const sx = col * this.frameWidth;
        const sy = row * this.frameHeight;
        
        // The window size is now 320x240, but the sprite aspect ratio is 320x180.
        // We maintain the aspect ratio and anchor the cat to the bottom of the window
        // to give the speech bubble 60px of safe space at the top.
        const spriteAspectRatio = 320 / 180;
        const dw = this.canvas.width;
        const dh = dw / spriteAspectRatio;
        
        const dx = 0;
        const dy = this.canvas.height - dh;
        
        this.ctx.drawImage(
            this.spriteImage,
            sx, sy, this.frameWidth, this.frameHeight,
            dx, dy, dw, dh
        );
    }

    showBubble(userInstruction, currentActivity, isCompleted, duration) {
        // Simple implementation: Dispatch custom event for BubbleOverlay to handle
        window.dispatchEvent(new CustomEvent('pet-bubble', { detail: { userInstruction, currentActivity, isCompleted, duration } }));
    }

    destroy() {
        if (this.animationId) {
            cancelAnimationFrame(this.animationId);
        }
        if (this.canvas && this.canvas.parentNode) {
            this.canvas.parentNode.removeChild(this.canvas);
        }
    }
}
