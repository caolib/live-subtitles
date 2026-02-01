import { onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useAppConfig(settingsStore) {
    const appWindow = getCurrentWindow();
    let unlistenSettingsSync = null;
    let unlistenModelSwitched = null;
    let unlistenClose = null;

    // 监听字体家族变化，更新 CSS 变量
    watch(
        () => settingsStore.subtitleFontFamily,
        (newVal) => {
            if (newVal) {
                document.documentElement.style.setProperty('--main-font-family', newVal);
            } else {
                document.documentElement.style.removeProperty('--main-font-family');
            }
        },
        { immediate: true }
    );

    // 同步模型配置到后端
    async function syncModelConfigToBackend() {
        const currentModel = settingsStore.currentModel;

        // 对于 Windows 语音识别引擎，不需要本地模型
        if (settingsStore.asrEngine === 'windowsspeech') {
            try {
                // 获取当前音频源对应的设备ID
                const currentDeviceId = settingsStore.audioSourceType === 'systemaudio'
                    ? settingsStore.audioDeviceIdForSystem
                    : settingsStore.audioDeviceIdForMicrophone;

                const config = {
                    asr_engine: "windowsspeech",
                    current_model_id: "",
                    models: [],
                    audio_source_type: settingsStore.audioSourceType,
                    audio_device_id: currentDeviceId || "",
                    windows_speech_language: {
                        language_tag: settingsStore.windowsSpeechLanguage || "zh-CN",
                        display_name: settingsStore.windowsSpeechLanguage || "中文",
                    },
                };
                await invoke("update_config", { config });
                console.log("Windows Speech config synced to backend, language:", settingsStore.windowsSpeechLanguage);
                return;
            } catch (e) {
                console.error("Failed to sync Windows Speech config:", e);
                return;
            }
        }

        // Sherpa-ONNX 需要模型
        if (!currentModel) {
            console.log("No model configured, skipping sync");
            return;
        }

        try {
            // 获取当前音频源对应的设备ID
            const currentDeviceId = settingsStore.audioSourceType === 'systemaudio'
                ? settingsStore.audioDeviceIdForSystem
                : settingsStore.audioDeviceIdForMicrophone;

            const config = {
                asr_engine: "sherpaonnx",
                current_model_id: currentModel.id,
                models: [{
                    id: currentModel.id,
                    name: currentModel.model_name,
                    model_dir: currentModel.model_dir,
                    model_type: {
                        type: "Transducer",
                        encoder: currentModel.encoder || "",
                        decoder: currentModel.decoder || "",
                        joiner: currentModel.joiner || "",
                    },
                    tokens: currentModel.tokens || "",
                    languages: ["zh", "en"],
                    sample_rate: 16000,
                    num_threads: 2,
                }],
                // 同步音频源配置
                audio_source_type: settingsStore.audioSourceType,
                audio_device_id: currentDeviceId || "",
                windows_speech_language: {
                    language_tag: "zh-CN",
                    display_name: "中文",
                },
            };
            await invoke("update_config", { config });
            console.log("Model config synced to backend:", currentModel.model_name);
            console.log("Audio source synced:", settingsStore.audioSourceType, "Device:", currentDeviceId || "Default");
        } catch (e) {
            console.error("Failed to sync model config:", e);
        }
    }

    // 打开设置窗口
    async function openSettings() {
        try {
            await invoke("open_settings");
        } catch (e) {
            console.error("Failed to open settings:", e);
        }
    }

    onMounted(async () => {
        // 如果有持久化的模型配置，同步到 Rust 后端
        await syncModelConfigToBackend();

        // 监听设置同步事件（来自 Settings 窗口）
        unlistenSettingsSync = await appWindow.listen('settings-sync', (event) => {
            const appearance = event.payload;
            if (appearance) {
                // 更新本地 store
                settingsStore.updateAppearanceSettings(appearance);
                settingsStore.updateDisplaySettings(appearance); // showHistory 等在这里
            }
        });

        // 监听模型切换事件（从 Settings 窗口发送）
        unlistenModelSwitched = await appWindow.listen('model-switched', (event) => {
            console.log('Received model-switched event:', event.payload);
            // 注意：实际的状态同步由 main.js 中的 storage 事件监听器处理
        });

        // 拦截窗口关闭事件，改为隐藏窗口
        unlistenClose = await appWindow.onCloseRequested(async (event) => {
            event.preventDefault();
            await appWindow.hide();
        });
    });

    onUnmounted(() => {
        if (unlistenSettingsSync) unlistenSettingsSync();
        if (unlistenModelSwitched) unlistenModelSwitched();
        if (unlistenClose) unlistenClose();
    });

    return {
        syncModelConfigToBackend,
        openSettings
    };
}
