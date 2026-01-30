import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useSubtitle(settingsStore) {
    // 状态
    const isRunning = ref(false);
    const isModelLoading = ref(false); // 模型加载中
    const subtitles = ref([]); // 已完成的字幕历史
    const currentText = ref(""); // 正在识别的文本（中间结果）
    const errorMessage = ref("");

    const maxSubtitles = 5; // 最多显示的字幕条数
    const appWindow = getCurrentWindow();

    // 监听器
    let unlistenSubtitle = null;
    let unlistenError = null;
    let unlistenModelLoading = null;

    // 处理文本：根据配置决定是否转小写
    function processText(text) {
        if (!text) return text;
        // 如果启用了小写配置，将英文字母转换为小写
        if (settingsStore.lowercaseSubtitle) {
            return text.replace(/[A-Z]/g, char => char.toLowerCase());
        }
        return text;
    }

    // 切换大小写模式
    function toggleCase() {
        settingsStore.lowercaseSubtitle = !settingsStore.lowercaseSubtitle;
    }

    // 切换历史字幕显示
    function toggleHistory() {
        settingsStore.showHistory = !settingsStore.showHistory;
    }

    // 开始/停止识别
    async function toggleRecognition() {
        try {
            if (isRunning.value) {
                await invoke("stop_recognition");
                isRunning.value = false;
            } else {
                await invoke("start_recognition");
                isRunning.value = true;
                errorMessage.value = "";
            }
        } catch (e) {
            errorMessage.value = String(e);
            console.error("Recognition error:", e);
        }
    }

    // 清空字幕
    function clearSubtitles() {
        subtitles.value = [];
        currentText.value = "";
    }

    // 复制所有文本
    async function copyAllText() {
        const allText = [...subtitles.value.map(s => s.text), currentText.value]
            .filter(t => t && t.trim())
            .join('\n');
        if (allText) {
            try {
                await navigator.clipboard.writeText(allText);
            } catch (e) {
                console.error('Failed to copy:', e);
            }
        }
    }

    // 最新的字幕（正在识别的文本，或最后一条已完成的）
    const latestSubtitle = computed(() => {
        if (currentText.value) {
            return processText(currentText.value);
        }
        if (subtitles.value.length === 0) return "";
        return processText(subtitles.value[subtitles.value.length - 1].text);
    });

    // 历史字幕 (已完成的字幕，如果有正在识别的则显示全部，否则除了最后一条)
    const historySubtitles = computed(() => {
        if (!settingsStore.showHistory) return [];

        let history;
        if (currentText.value) {
            // 有正在识别的文本，显示所有已完成的字幕
            history = subtitles.value;
        } else {
            // 没有正在识别的文本，最后一条作为最新字幕显示
            if (subtitles.value.length <= 1) return [];
            history = subtitles.value.slice(0, -1);
        }
        return history;
    });

    // 历史字幕文本（带长度限制）
    const historyText = computed(() => {
        const text = historySubtitles.value.map(s => processText(s.text)).join(' ');
        if (settingsStore.maxHistoryLength > 0 && text.length > settingsStore.maxHistoryLength) {
            return '...' + text.slice(-settingsStore.maxHistoryLength);
        }
        return text;
    });

    onMounted(async () => {
        // 检查初始状态
        try {
            isRunning.value = await invoke("is_recognition_running");
        } catch (e) {
            console.error("Failed to get initial state:", e);
        }

        // 监听字幕事件
        unlistenSubtitle = await listen("subtitle", (event) => {
            const subtitle = event.payload;
            if (subtitle.text && subtitle.text.trim()) {
                if (subtitle.is_final) {
                    // 句子结束，添加到历史记录
                    subtitles.value.push({
                        id: Date.now(),
                        text: subtitle.text,
                        timestamp: subtitle.timestamp,
                    });
                    // 保持最大条数
                    if (subtitles.value.length > maxSubtitles) {
                        subtitles.value.shift();
                    }
                    // 清空当前正在识别的文本
                    currentText.value = "";
                } else {
                    // 中间结果，更新当前正在识别的文本（替换而不是追加）
                    currentText.value = subtitle.text;
                }
            }
        });

        // 监听错误事件
        unlistenError = await listen("recognition_error", (event) => {
            errorMessage.value = String(event.payload);
            isRunning.value = false;
            isModelLoading.value = false;
        });

        // 监听模型加载状态
        unlistenModelLoading = await listen("model_loading", (event) => {
            isModelLoading.value = event.payload.loading;
        });

        // 自动开始识别（仅在有配置的情况下）
        if (!isRunning.value && settingsStore.currentModel) {
            try {
                await invoke("start_recognition");
                isRunning.value = true;
            } catch (e) {
                // 如果启动失败，可能是配置问题
                errorMessage.value = String(e);
                console.error("Auto start failed:", e);
                // 不是致命错误，用户可以手动启动
            }
        } else if (!settingsStore.currentModel) {
            console.log("No model configured, skipping auto start");
        }
    });

    onUnmounted(() => {
        if (unlistenSubtitle) unlistenSubtitle();
        if (unlistenError) unlistenError();
        if (unlistenModelLoading) unlistenModelLoading();
    });

    return {
        isRunning,
        isModelLoading,
        subtitles,
        currentText,
        errorMessage,
        latestSubtitle,
        historySubtitles,
        historyText,
        toggleRecognition,
        clearSubtitles,
        copyAllText,
        toggleCase,
        toggleHistory
    };
}
