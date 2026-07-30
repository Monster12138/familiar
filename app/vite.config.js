import { defineConfig } from 'vite';

export default defineConfig({
  root: 'src',
  publicDir: '../public',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: 'src/index.html',
        settings: 'src/settings.html',
        bubble: 'src/bubble.html',
        stats: 'src/stats.html'
      }
    }
  },
  server: {
    port: 5173,
    strictPort: true,
    fs: {
      allow: ['../..']
    }
  }
});
