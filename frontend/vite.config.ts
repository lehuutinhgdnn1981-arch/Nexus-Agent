import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@bindings': path.resolve(__dirname, './src/bindings'),
      '@store': path.resolve(__dirname, './src/store'),
      '@components': path.resolve(__dirname, './src/components'),
      '@lib': path.resolve(__dirname, './src/lib'),
      '@hooks': path.resolve(__dirname, './src/hooks'),
    },
  },
  // Tauri expects a fixed port; if not available, fall back.
  server: {
    port: 5173,
    strictPort: false,
    watch: {
      // Don't watch backend changes
      ignored: ['**/src-tauri/**', '**/target/**'],
    },
  },
  // Tauri uses fixed port 5173 in production mode (custom protocol)
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: false,
  },
});
