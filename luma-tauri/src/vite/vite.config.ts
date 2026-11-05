import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";
import { fileURLToPath } from "url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  plugins: [vue()],

  root: __dirname,

  server: {
    port: 1420,
    strictPort: true,
  },

  build: {
    outDir: resolve(__dirname, "../../../vite_dist"),
    emptyOutDir: true,
    rolldownOptions: {
      input: {
        main: resolve(__dirname, "windows" ,"index.html"),
        settings: resolve(__dirname, "windows", "settings.html"),
        startup: resolve(__dirname, "windows", "startup.html")
      }
    }
  },
});