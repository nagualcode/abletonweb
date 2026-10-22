import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';

// In dev, the bridge (the installed service or `cargo run -- serve`, port 6090)
// handles the WebSocket.
export default defineConfig({
  plugins: [svelte()],
  server: {
    host: true,
    proxy: { '/ws': { target: 'ws://127.0.0.1:6090', ws: true } },
  },
});
