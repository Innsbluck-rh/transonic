import tailwindcss from '@tailwindcss/vite';
import path from 'path';
import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import webfontDownload from 'vite-plugin-webfont-dl';

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [
    solid(),
    tailwindcss(),
    webfontDownload([
      'https://fonts.googleapis.com/css2?family=Archivo+Black&family=DotGothic16&family=Public+Sans:ital,wght@0,100..900;1,100..900&display=swap',
    ]),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
  resolve: {
    alias: {
      '~': path.join(__dirname, 'src'),
    },
  },
}));
