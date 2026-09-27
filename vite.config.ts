import { defineConfig } from "vite";
import { resolve } from "node:path";

// The overlay page is the only document the app has - the rest of the UI is
// the tray menu, which is built in Rust. It is not index.html because it is
// not a page anyone navigates to; naming it overlay.html keeps that clear.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    rollupOptions: {
      input: { overlay: resolve(__dirname, "overlay.html") },
    },
  },
});
