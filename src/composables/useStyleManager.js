import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { readTextFile } from "@tauri-apps/plugin-fs";

export function useStyleManager() {
    const customStyleElement = ref(null);
    const stylePath = ref("");
    let styleWatchInterval = null;
    let lastStyleContent = "";

    // 应用自定义样式
    function applyCustomStyle(cssContent) {
        // 移除旧的样式元素
        if (customStyleElement.value) {
            customStyleElement.value.remove();
        }
        // 创建新的样式元素
        const style = document.createElement("style");
        style.id = "custom-subtitle-style";
        style.textContent = cssContent;
        document.head.appendChild(style);
        customStyleElement.value = style;
    }

    // 加载外部 CSS 样式
    async function loadCustomStyle() {
        try {
            stylePath.value = await invoke("get_style_path");
            const cssContent = await readTextFile(stylePath.value);
            applyCustomStyle(cssContent);
        } catch (e) {
            console.error("Failed to load custom style:", e);
        }
    }

    // 监听样式文件变化（使用轮询方式）
    async function watchStyleFile() {
        if (!stylePath.value) return;

        try {
            // 记录初始内容
            lastStyleContent = await readTextFile(stylePath.value);

            // 每2秒检查一次文件变化
            styleWatchInterval = setInterval(async () => {
                try {
                    const currentContent = await readTextFile(stylePath.value);
                    if (currentContent !== lastStyleContent) {
                        console.log("Style file changed, reloading...");
                        lastStyleContent = currentContent;
                        applyCustomStyle(currentContent);
                    }
                } catch (e) {
                    // 文件可能正在被写入，忽略错误
                }
            }, 2000);
        } catch (e) {
            console.error("Failed to setup style file watch:", e);
        }
    }

    // 打开样式编辑器
    async function openStyleEditor() {
        try {
            await invoke("open_style_editor");
        } catch (e) {
            console.error("Failed to open style editor:", e);
        }
    }

    onMounted(async () => {
        // 加载自定义样式
        await loadCustomStyle();
        // 监听样式文件变化
        await watchStyleFile();
    });

    onUnmounted(() => {
        // 清理样式文件监听
        if (styleWatchInterval) clearInterval(styleWatchInterval);
        // 移除自定义样式元素
        if (customStyleElement.value) {
            customStyleElement.value.remove();
        }
    });

    return {
        customStyleElement,
        stylePath,
        loadCustomStyle,
        openStyleEditor
    };
}
