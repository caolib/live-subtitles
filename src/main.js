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
// 使用防抖和同步标志避免无限循环
let syncTimeout = null;
let isSyncing = false;

window.addEventListener('storage', (e) => {
    // 只处理 settings store 的变化，并且不在同步过程中
    if (e.key === 'live-subtitles-settings' && e.newValue && !isSyncing) {
        // 清除之前的定时器，实现防抖
        if (syncTimeout) {
            clearTimeout(syncTimeout);
        }

        // 300ms 防抖
        syncTimeout = setTimeout(() => {
            try {
                isSyncing = true;
                const newState = JSON.parse(e.newValue);
                const settingsStore = useSettingsStore();

                // 静默日志，只在开发环境输出
                if (import.meta.env.DEV) {
                    console.log('[Storage Sync] Syncing from another window');
                }

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
                if (newState.audioSourceType !== undefined) {
                    settingsStore.audioSourceType = newState.audioSourceType;
                }
                if (newState.audioDeviceId !== undefined) {
                    settingsStore.audioDeviceId = newState.audioDeviceId;
                }
                if (newState.audioDeviceIdForSystem !== undefined) {
                    settingsStore.audioDeviceIdForSystem = newState.audioDeviceIdForSystem;
                }
                if (newState.audioDeviceIdForMicrophone !== undefined) {
                    settingsStore.audioDeviceIdForMicrophone = newState.audioDeviceIdForMicrophone;
                }
                if (newState.useCustomProxy !== undefined) {
                    settingsStore.useCustomProxy = newState.useCustomProxy;
                }
                if (newState.proxyUrl !== undefined) {
                    settingsStore.proxyUrl = newState.proxyUrl;
                }
                if (newState.proxyUsername !== undefined) {
                    settingsStore.proxyUsername = newState.proxyUsername;
                }
                if (newState.proxyPassword !== undefined) {
                    settingsStore.proxyPassword = newState.proxyPassword;
                }
            } catch (err) {
                console.error('Failed to sync Pinia state from storage:', err);
            } finally {
                // 延迟重置同步标志，避免立即触发
                setTimeout(() => {
                    isSyncing = false;
                }, 100);
            }
        }, 300);
    }
});

app.mount('#app')