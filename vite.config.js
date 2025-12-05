import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import vueDevTools from "vite-plugin-vue-devtools";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
    plugins: [vue(), vueDevTools()],
    clearScreen: false,
    build: {
        rollupOptions: {
            input: {
                main: resolve(__dirname, "index.html"),
                settings: resolve(__dirname, "settings.html"),
            },
        },
    },
    server: {
        port: 1420,
        strictPort: true,
        host: host || "localhost",
        hmr: host
            ? {
                protocol: "ws",
                host,
                port: 1421,
            }
            : undefined,
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
});
