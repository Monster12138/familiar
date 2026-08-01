import { SpriteRenderer } from './SpriteRenderer.js';
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { convertFileSrc } from "@tauri-apps/api/core";

export class PixelSpriteRenderer extends SpriteRenderer {
    constructor() {
        super();
        this.container = null;
        this.imgElement = null;
        this.offscreenCanvas = null;
        this.offscreenCtx = null;
        
        this.packInfo = null;
        this.manifest = null;
        this.currentAnim = null;
        this.currentAssetUrl = null;
    }

    init(container) {
        this.container = container;
        container.innerHTML = '';

        // Image element for state asset (PNG, GIF, WEBP, SVG, APNG)
        this.imgElement = document.createElement('img');
        this.imgElement.style.display = 'block';
        this.imgElement.style.imageRendering = 'pixelated';
        this.imgElement.style.userSelect = 'none';
        this.imgElement.style.webkitUserDrag = 'none';

        // Offscreen canvas for alpha-channel click testing
        this.offscreenCanvas = document.createElement('canvas');
        this.offscreenCtx = this.offscreenCanvas.getContext('2d', { willReadFrequently: true });

        this.imgElement.addEventListener('mousedown', (e) => {
            if (e.button !== 0) return;
            const rect = this.imgElement.getBoundingClientRect();
            if (rect.width === 0 || rect.height === 0) return;

            this.offscreenCanvas.width = this.imgElement.naturalWidth || rect.width;
            this.offscreenCanvas.height = this.imgElement.naturalHeight || rect.height;
            try {
                this.offscreenCtx.clearRect(0, 0, this.offscreenCanvas.width, this.offscreenCanvas.height);
                this.offscreenCtx.drawImage(this.imgElement, 0, 0);
                const scaleX = this.offscreenCanvas.width / rect.width;
                const scaleY = this.offscreenCanvas.height / rect.height;
                const x = (e.clientX - rect.left) * scaleX;
                const y = (e.clientY - rect.top) * scaleY;
                const pixel = this.offscreenCtx.getImageData(x, y, 1, 1).data;
                if (pixel[3] > 0) {
                    getCurrentWebviewWindow().startDragging();
                }
            } catch (err) {
                getCurrentWebviewWindow().startDragging();
            }
        });

        container.appendChild(this.imgElement);
    }

    async loadSpritePack(packInfoOrManifest) {
        if (packInfoOrManifest && packInfoOrManifest.manifest) {
            this.packInfo = packInfoOrManifest;
            this.manifest = packInfoOrManifest.manifest;
        } else {
            this.manifest = packInfoOrManifest || {};
            this.packInfo = { manifest: this.manifest, is_builtin: true };
        }

        const states = this.manifest.states || {};
        const idleFile = states.idle || states.default || Object.values(states)[0] || 'idle.png';
        const idleUrl = this._resolveAssetUrl(idleFile);

        this.currentAnim = 'idle';
        this.currentAssetUrl = idleUrl;

        return new Promise((resolve) => {
            const tempImg = new Image();
            tempImg.onload = () => {
                this._resizeElement(this.imgElement, tempImg.naturalWidth, tempImg.naturalHeight);
                this.imgElement.src = idleUrl;
                resolve();
            };
            tempImg.onerror = () => {
                this.imgElement.src = idleUrl;
                resolve();
            };
            tempImg.src = idleUrl;
        });
    }

    _resolveAssetUrl(fileName) {
        if (!fileName) return '';
        if (fileName.startsWith('http://') || fileName.startsWith('https://') || fileName.startsWith('data:')) {
            return fileName;
        }

        if (this.packInfo && this.packInfo.state_urls && this.packInfo.state_urls[fileName]) {
            return this.packInfo.state_urls[fileName];
        }

        const packId = this.manifest.id || this.manifest.name || 'british-blue';
        if (this.packInfo && !this.packInfo.is_builtin && this.packInfo.path) {
            try {
                return convertFileSrc(`${this.packInfo.path}/${fileName}`);
            } catch (e) {
                return `/sprites/${packId}/${fileName}`;
            }
        }

        return `/sprites/${packId}/${fileName}`;
    }

    _resizeElement(element, width, height) {
        const baseRenderSize = 128;
        const aspectRatio = (width && height) ? (width / height) : 1;
        let cssWidth, cssHeight;
        if (aspectRatio > 1) {
            cssWidth = baseRenderSize;
            cssHeight = baseRenderSize / aspectRatio;
        } else {
            cssHeight = baseRenderSize;
            cssWidth = baseRenderSize * aspectRatio;
        }
        element.style.width = `${cssWidth}px`;
        element.style.height = `${cssHeight}px`;
    }

    playAnimation(name) {
        if (this.currentAnim === name && this.currentAssetUrl) {
            return;
        }

        const states = this.manifest ? (this.manifest.states || {}) : {};
        let file = states[name];

        if (!file) {
            if (name === 'interacting') file = states.happy || states.idle;
            else if (name === 'happy') file = states.interacting || states.idle;
            else if (name === 'celebrating') file = states.happy || states.interacting || states.idle;
            else if (name === 'watching') file = states.thinking || states.idle;
            else file = states.idle || states.default || Object.values(states)[0];
        }

        if (file) {
            const targetUrl = this._resolveAssetUrl(file);
            if (this.currentAssetUrl === targetUrl) {
                this.currentAnim = name;
                return;
            }

            this.currentAnim = name;
            const tempImg = new Image();
            tempImg.onload = () => {
                this._resizeElement(this.imgElement, tempImg.naturalWidth, tempImg.naturalHeight);
                this.imgElement.src = targetUrl;
                this.currentAssetUrl = targetUrl;
            };
            tempImg.onerror = () => {
                this.imgElement.src = targetUrl;
                this.currentAssetUrl = targetUrl;
            };
            tempImg.src = targetUrl;
        }
    }

    destroy() {
        if (this.imgElement && this.imgElement.parentNode) {
            this.imgElement.parentNode.removeChild(this.imgElement);
        }
    }
}
