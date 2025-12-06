import { createApp } from "vue";
import { RouterView } from "vue-router";
import router from "./router";
import pinia from "./stores";
import { useSettingsStore } from "./stores/settings";
import Antd from "ant-design-vue";
import "ant-design-vue/dist/reset.css";
import "./assets/styles/var.css";

// 禁用生产环境的右键上下文菜单
if (import.meta.env.PROD) {
    document.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        return false;
    }, { capture: true });
}

// 创建应用并使用 RouterView 作为根组件
const app = createApp(RouterView);

app.use(pinia);
app.use(router);
app.use(Antd);

// 监听 storage 事件，实现多窗口 Pinia 状态同步
window.addEventListener('storage', (e) => {
    // 只处理 settings store 的变化
    if (e.key === 'live-subtitles-settings' && e.newValue) {
        try {
            const newState = JSON.parse(e.newValue);
            const settingsStore = useSettingsStore();

            console.log('Storage changed, syncing Pinia state:', newState);

            // 手动同步所有需要的字段
            if (newState.currentModelId !== undefined) {
                settingsStore.currentModelId = newState.currentModelId;
            }
            if (newState.availableModels !== undefined) {
                settingsStore.availableModels = newState.availableModels;
            }
            if (newState.modelsRootDir !== undefined) {
                settingsStore.modelsRootDir = newState.modelsRootDir;
            }
            if (newState.modelAdvancedConfig !== undefined) {
                settingsStore.modelAdvancedConfig = newState.modelAdvancedConfig;
            }
            if (newState.showHistory !== undefined) {
                settingsStore.showHistory = newState.showHistory;
            }
            if (newState.maxHistoryLength !== undefined) {
                settingsStore.maxHistoryLength = newState.maxHistoryLength;
            }
            if (newState.themeMode !== undefined) {
                settingsStore.themeMode = newState.themeMode;
            }
            if (newState.rememberWindowState !== undefined) {
                settingsStore.rememberWindowState = newState.rememberWindowState;
            }

            console.log('Pinia state synced, current model ID:', settingsStore.currentModelId);
        } catch (err) {
            console.error('Failed to sync Pinia state from storage:', err);
        }
    }
});

app.mount('#app')