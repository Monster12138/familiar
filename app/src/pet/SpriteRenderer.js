/**
 * SpriteRenderer Interface
 * Extensibility point for different rendering backends (Canvas2D, WebGL, Live2D, etc.)
 */
export class SpriteRenderer {
    /**
     * @param {HTMLElement} container The container to append the renderer to
     */
    init(container) {
        throw new Error("Not implemented");
    }

    /**
     * @param {Object} manifest The sprite pack manifest JSON
     */
    async loadSpritePack(manifest) {
        throw new Error("Not implemented");
    }

    /**
     * @param {string} name The animation name to play
     */
    playAnimation(name) {
        throw new Error("Not implemented");
    }

    /**
     * @param {string} text The text to show in the bubble
     * @param {number} duration Duration in ms
     */
    showBubble(text, duration) {
        throw new Error("Not implemented");
    }

    destroy() {
        throw new Error("Not implemented");
    }
}
