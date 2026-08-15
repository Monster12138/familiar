import { defineConfig } from 'vite';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const appDir = path.dirname(fileURLToPath(import.meta.url));
const spritesDir = path.resolve(appDir, '../sprites');

// Serve the sprite packs straight from the repository root. The previous
// `app/public/sprites` symlink breaks on Windows checkouts where git
// materializes symlinks as plain text files (core.symlinks=false).
function repoSprites() {
  // Resolved by Vite (relative to `root`), so the copy lands next to the
  // built frontend instead of next to this config file.
  let outDir = '';
  return {
    name: 'repo-sprites',
    configResolved(config) {
      // `outDir` may be relative to `root`, resolve it to an absolute path.
      outDir = path.resolve(config.root, config.build.outDir);
    },
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (!req.url || !req.url.startsWith('/sprites/')) return next();
        const rel = decodeURIComponent(req.url.slice('/sprites/'.length).split('?')[0]);
        const file = path.normalize(path.join(spritesDir, rel));
        const inside = file === spritesDir || file.startsWith(spritesDir + path.sep);
        if (!inside || !fs.existsSync(file) || !fs.statSync(file).isFile()) {
          return next();
        }
        const type = file.endsWith('.json')
          ? 'application/json'
          : file.endsWith('.png')
            ? 'image/png'
            : 'application/octet-stream';
        res.setHeader('Content-Type', type);
        fs.createReadStream(file).pipe(res);
      });
    },
    closeBundle() {
      if (outDir) {
        fs.cpSync(spritesDir, path.join(outDir, 'sprites'), { recursive: true });
      }
    }
  };
}

export default defineConfig({
  root: 'src',
  plugins: [repoSprites()],
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: 'src/index.html',
        settings: 'src/settings.html',
        bubble: 'src/bubble.html',
        stats: 'src/stats.html',
        onboard: 'src/onboard.html'
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
