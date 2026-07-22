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
        this.canvas.style.display = 'block';
        this.canvas.style.imageRendering = 'pixelated';
        
        this.ctx = this.canvas.getContext('2d');
        // Ensure crisp pixels
        this.ctx.imageSmoothingEnabled = false;

        // Pixel-perfect drag using canvas alpha channel
        this.canvas.addEventListener('mousedown', (e) => {
            if (e.button === 0) {
                // Get pixel data at the click coordinates
                const rect = this.canvas.getBoundingClientRect();
                const scaleX = this.canvas.width / rect.width;
                const scaleY = this.canvas.height / rect.height;
                const x = (e.clientX - rect.left) * scaleX;
                const y = (e.clientY - rect.top) * scaleY;
                
                const pixel = this.ctx.getImageData(x, y, 1, 1).data;
                if (pixel[3] > 0) { // If alpha > 0 (not transparent)
                    import("@tauri-apps/api/webviewWindow").then(({ getCurrentWebviewWindow }) => {
                        getCurrentWebviewWindow().startDragging();
                    });
                }
            }
        });

        // Add canvas to DOM
        container.appendChild(this.canvas);
        
        // No window resize listener needed, unified container handles scale
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
                
                this.canvas.width = this.frameWidth;
                this.canvas.height = this.frameHeight;
                
                // Normalize the pet size to a base CSS box of 128x128, preserving aspect ratio.
                // This prevents massive AI-generated sprites from becoming gigantic.
                const baseRenderSize = 128;
                const aspectRatio = this.frameWidth / this.frameHeight;
                let cssWidth, cssHeight;
                if (aspectRatio > 1) {
                    cssWidth = baseRenderSize;
                    cssHeight = baseRenderSize / aspectRatio;
                } else {
                    cssHeight = baseRenderSize;
                    cssWidth = baseRenderSize * aspectRatio;
                }
                
                this.canvas.style.width = `${cssWidth}px`;
                this.canvas.style.height = `${cssHeight}px`;
                
                this.ctx.imageSmoothingEnabled = false;
                resolve();
            };
            this.spriteImage.onerror = reject;
            // The image is expected to be served statically
            this.spriteImage.src = `/sprites/${manifest.name}/${manifest.sprite_sheet}`;
        });
    }
    
    // resizeWindowToFrame removed because window size is managed by main.js
    


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
        
        const dw = this.canvas.width;
        const dh = this.canvas.height;
        
        this.ctx.drawImage(
            this.spriteImage,
            sx, sy, this.frameWidth, this.frameHeight,
            0, 0, dw, dh
        );
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
