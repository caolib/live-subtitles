import { defineConfig } from "@rsbuild/core";
import { pluginVue } from "@rsbuild/plugin-vue";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
    plugins: [pluginVue()],
    html: {
        template: "./index.html",
    },
    source: {
        entry: {
            index: "./src/main.js",
        },
    },
    server: {
        port: 1420,
        strictPort: true,
        host: host || "localhost",
    },
    dev: {
        hmr: host
            ? {
                protocol: "ws",
                host,
                port: 1421,
            }
            : true,
        watchFiles: {
            // 忽略 src-tauri 目录
            paths: ["src/**", "public/**"],
        },
    },
    output: {
        distPath: {
            root: "dist",
        },
    },
});
