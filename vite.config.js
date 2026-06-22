import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      external: [],
    },
  },
  server: {
    port: 5173,
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
  resolve: {
    preserveSymlinks: true,
  },
});