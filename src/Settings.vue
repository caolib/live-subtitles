<script setup>
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { documentDir } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
import { theme, message } from "ant-design-vue";
import {
    FolderOpenOutlined,
    DownloadOutlined,
    UploadOutlined,
    ReloadOutlined,
    SaveOutlined,
    MinusOutlined,
    CloseOutlined,
    ScanOutlined,
    CheckCircleOutlined,
    CloseCircleOutlined,
    InfoCircleOutlined,
    SyncOutlined
} from "@ant-design/icons-vue";
import { useSettingsStore } from "./stores/settings";

// 窗口操作
const appWindow = getCurrentWindow();

async function minimizeWindow() {
    await appWindow.minimize();
}

async function closeWindow() {
    await appWindow.hide();
}

function startDrag() {
    appWindow.startDragging();
}

// Pinia Store
const settingsStore = useSettingsStore();

// 主题选项
const themeOptions = [
    { label: "跟随系统", value: "system" },
    { label: "浅色", value: "light" },
    { label: "深色", value: "dark" },
];

// 系统主题检测
const systemDark = ref(window.matchMedia("(prefers-color-scheme: dark)").matches);

// 监听系统主题变化
onMounted(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e) => { systemDark.value = e.matches; };
    mediaQuery.addEventListener("change", handler);
});

// 计算实际使用的主题算法
const themeAlgorithm = computed(() => {
    if (settingsStore.themeMode === "system") {
        return systemDark.value ? theme.darkAlgorithm : theme.defaultAlgorithm;
    }
    return settingsStore.themeMode === "dark" ? theme.darkAlgorithm : theme.defaultAlgorithm;
});

// 计算是否为深色模式
const isDark = computed(() => {
    if (settingsStore.themeMode === "system") {
        return systemDark.value;
    }
    return settingsStore.themeMode === "dark";
});

// 默认模型目录（从 Rust 获取）
const defaultModelsDir = ref("");

// 当前选中模型的手动配置
const currentModelAdvancedForm = ref({
    encoder: "",
    decoder: "",
    joiner: "",
    tokens: "",
});

const loading = ref(false);
const scanning = ref(false);

// 当前选中的模型详情
const currentModelDetails = computed(() => {
    return settingsStore.availableModels.find(m => m.id === settingsStore.currentModelId);
});

// 当前选中的模型版本（int8 或 fp32）
const selectedVariant = ref("int8");

// 关于对话框显示状态
const aboutVisible = ref(false);

// 音频设备加载状态
const loadingAudioDevices = ref(false);

// 音频源类型选项
const audioSourceTypeOptions = [
    { label: "系统音频", value: "systemaudio" },
    { label: "麦克风输入", value: "microphone" },
];
const appVersion = ref('加载中...');
const hasUpdate = ref(false);
const latestVersion = ref('');
const updateNotes = ref('');
const checkingUpdate = ref(false);
const downloading = ref(false);
const downloadProgress = ref(0);

// 检查当前模型配置是否完整
const isModelComplete = computed(() => {
    return currentModelAdvancedForm.value.encoder &&
        currentModelAdvancedForm.value.decoder &&
        currentModelAdvancedForm.value.joiner &&
        currentModelAdvancedForm.value.tokens;
});

// 获取模型目录前缀
const modelDirPrefix = computed(() => {
    if (!currentModelDetails.value) return '';
    return currentModelDetails.value.model_dir + '\\';
});

// 检查文件路径是否是自定义的（不在模型目录下）
function isCustomPath(filePath) {
    if (!filePath || !modelDirPrefix.value) return false;
    return !filePath.startsWith(modelDirPrefix.value);
}

// 获取显示的文件名（如果在模型目录下只显示文件名）
function getDisplayFileName(filePath) {
    if (!filePath) return '';
    if (isCustomPath(filePath)) {
        return filePath; // 自定义路径显示完整路径
    }
    // 在模型目录下，只显示文件名
    const parts = filePath.split(/[\\/]/);
    return parts[parts.length - 1];
}

// 加载配置
async function loadConfig() {
    try {
        // 获取默认模型目录
        defaultModelsDir.value = await invoke("get_models_dir");

        // 如果 Pinia 中没有保存过根目录，使用默认目录
        if (!settingsStore.modelsRootDir) {
            settingsStore.modelsRootDir = defaultModelsDir.value;
        }

        // 自动扫描模型
        await scanModelsRootDir();

        // 加载当前模型的高级配置
        loadCurrentModelAdvancedConfig();
    } catch (e) {
        console.error("Failed to load config:", e);
    }
}

// 加载当前模型的高级配置到表单
function loadCurrentModelAdvancedConfig() {
    const currentModel = currentModelDetails.value;
    if (currentModel) {
        // 如果有多个版本，初始化 selectedVariant
        if (currentModel.has_multiple_variants && currentModel.variants?.length > 0) {
            // 默认选择 int8 版本（如果存在）
            const hasInt8 = currentModel.variants.some(v => v.variant_name === 'int8');
            selectedVariant.value = hasInt8 ? 'int8' : currentModel.variants[0].variant_name;
        }

        // 从选中的版本加载配置
        loadVariantConfig();
    }
}

// 从当前选中的版本加载配置
function loadVariantConfig() {
    const advancedConfig = settingsStore.modelAdvancedConfig[settingsStore.currentModelId];
    const scannedModel = currentModelDetails.value;

    let encoder = "", decoder = "", joiner = "";

    // 如果有多个版本，从选中的版本加载
    if (scannedModel?.has_multiple_variants && scannedModel.variants?.length > 0) {
        const variant = scannedModel.variants.find(v => v.variant_name === selectedVariant.value);
        if (variant) {
            encoder = variant.encoder;
            decoder = variant.decoder;
            joiner = variant.joiner;
        }
    } else {
        // 单版本：使用扫描到的默认值
        encoder = scannedModel?.encoder || "";
        decoder = scannedModel?.decoder || "";
        joiner = scannedModel?.joiner || "";
    }

    // 版本切换时不使用高级配置覆盖，确保路径更新
    // 只有 tokens 使用高级配置（因为 tokens 文件是共用的）
    currentModelAdvancedForm.value = {
        encoder: encoder,
        decoder: decoder,
        joiner: joiner,
        tokens: advancedConfig?.tokens || scannedModel?.tokens || "",
    };
}

// 切换模型版本
async function switchModelVariant(variantName) {
    console.log('Switching model variant to:', variantName);
    selectedVariant.value = variantName;

    // 重新加载配置（会更新 currentModelAdvancedForm）
    loadVariantConfig();

    // 保存到高级配置（保存新版本的路径）
    settingsStore.modelAdvancedConfig[settingsStore.currentModelId] = {
        encoder: currentModelAdvancedForm.value.encoder,
        decoder: currentModelAdvancedForm.value.decoder,
        joiner: currentModelAdvancedForm.value.joiner,
        tokens: currentModelAdvancedForm.value.tokens,
    };

    // 检查识别是否正在运行
    const wasRunning = await invoke("is_recognition_running");

    // 同步模型配置到后端
    await syncModelToBackend();

    // 仅在识别运行时才停止并重启，暂停状态下只切换不启动
    if (wasRunning) {
        try {
            await invoke("stop_recognition");
            console.log("Recognition stopped for variant switch");
            await new Promise(resolve => setTimeout(resolve, 500));

            await invoke("start_recognition");
            console.log("Recognition restarted with new variant");
            message.success(`已切换到 ${variantName === 'int8' ? '快速版本（int8）' : '精确版本（fp32）'}`);
        } catch (e) {
            console.error("Failed to restart recognition:", e);
            message.warning(`版本已切换，但自动重启失败: ${e}。请手动点击开始按钮。`);
        }
    } else {
        message.success(`已切换到 ${variantName === 'int8' ? '快速版本（int8）' : '精确版本（fp32）'}，下次启动时生效`);
    }
}

// 同步当前模型配置到后端
async function syncModelToBackend() {
    const currentModel = settingsStore.currentModel;
    if (!currentModel) return;

    try {
        const updatedConfig = {
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
            audio_device_id: currentAudioDeviceId.value || "",
        };
        await invoke("update_config", { config: updatedConfig });
        console.log("Model synced to backend:", currentModel.model_name);
    } catch (e) {
        console.error("Failed to sync model to backend:", e);
    }
}

// 监听当前模型变化，加载对应的高级配置并同步到后端
watch(() => settingsStore.currentModelId, async (newId, oldId) => {
    console.log('Watch triggered - currentModelId changed:', { oldId, newId });
    loadCurrentModelAdvancedConfig();
    // 只有在实际切换模型时才同步（排除初始加载）
    if (oldId && newId && oldId !== newId) {
        const newModelName = settingsStore.getCurrentModelSync()?.model_name;
        console.log(`Switching model from ${oldId} to ${newId} (${newModelName})`);

        try {
            // 1. 检查识别是否在运行
            let wasRunning = false;
            try {
                wasRunning = await invoke("is_recognition_running");
                console.log(`Recognition is ${wasRunning ? 'running' : 'stopped'}`);
            } catch (e) {
                console.warn("Failed to check recognition status:", e);
            }

            // 2. 如果正在运行，先停止
            if (wasRunning) {
                console.log("Stopping recognition before switching model...");
                try {
                    await invoke("stop_recognition");
                    // 等待更长时间确保完全停止
                    await new Promise(resolve => setTimeout(resolve, 1000));
                    console.log("Recognition stopped");
                } catch (e) {
                    console.error("Failed to stop recognition:", e);
                    // 继续尝试切换配置
                }
            }

            // 3. 切换模型配置
            console.log("Syncing new model config to backend...");
            await syncModelToBackend();
            await new Promise(resolve => setTimeout(resolve, 500));

            // 4. 如果之前在运行，重新启动
            if (wasRunning) {
                console.log("Restarting recognition with new model...");
                try {
                    // 确保之前的识别已完全停止
                    const stillRunning = await invoke("is_recognition_running");
                    if (stillRunning) {
                        console.log("Recognition still running, waiting longer...");
                        await new Promise(resolve => setTimeout(resolve, 1000));
                    }

                    // 通知前端开始加载模型
                    await invoke("start_recognition");
                    console.log("Recognition restarted successfully");
                } catch (e) {
                    console.error("Failed to restart recognition:", e);
                    message.warning(`模型已切换，但自动重启失败: ${e}。请手动点击开始按钮。`);
                    return;
                }
            }

            message.success(`已切换到模型: ${newModelName}`);
            console.log(`Model switched successfully to: ${newModelName}`);

            // 发送模型切换事件给 App.vue
            try {
                await appWindow.emit('model-switched', { modelId: newId, modelName: newModelName });
                console.log('Emitted model-switched event');
            } catch (e) {
                console.error('Failed to emit model-switched event:', e);
            }
        } catch (e) {
            message.error(`模型切换失败: ${e}`);
            console.error("Failed to switch model:", e);
        }
    }
});

// 选择文件
async function selectFile(field) {
    try {
        const modelDir = currentModelDetails.value?.model_dir || settingsStore.modelsRootDir;
        const selected = await open({
            multiple: false,
            defaultPath: modelDir,
            filters: [
                {
                    name: field === "tokens" ? "Tokens" : "ONNX Model",
                    extensions: field === "tokens" ? ["txt"] : ["onnx"],
                },
            ],
        });
        if (selected) {
            currentModelAdvancedForm.value[field] = selected;
        }
    } catch (e) {
        console.error("Failed to select file:", e);
    }
}

// 选择模型根目录
async function selectModelsRootDir() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: settingsStore.modelsRootDir || defaultModelsDir.value,
        });
        if (selected) {
            const oldPath = settingsStore.modelsRootDir;
            settingsStore.modelsRootDir = selected;
            // 自动扫描模型
            await scanModelsRootDir();
            // 如果路径发生变化，提示用户重启
            if (oldPath && oldPath !== selected) {
                message.warning("模型文件夹路径已修改，建议重启应用以确保正常工作", 5);
            }
        }
    } catch (e) {
        console.error("Failed to select directory:", e);
    }
}

// 扫描模型根目录
async function scanModelsRootDir() {
    if (!settingsStore.modelsRootDir) {
        message.warning("请先设置模型根目录");
        return;
    }

    scanning.value = true;
    try {
        const models = await invoke("scan_models_root_dir", { rootDir: settingsStore.modelsRootDir });
        settingsStore.setAvailableModels(models);

        const completeCount = models.filter(m => m.is_complete).length;
        if (models.length === 0) {
            message.warning("未找到任何模型文件夹");
        }
    } catch (e) {
        message.error("扫描失败: " + e);
        console.error("Failed to scan models root dir:", e);
    } finally {
        scanning.value = false;
    }
}

onMounted(() => {
    loadConfig();
});

// 保存高级配置
function saveAdvancedConfig() {
    if (settingsStore.currentModelId) {
        settingsStore.setModelAdvancedConfig(settingsStore.currentModelId, {
            encoder: currentModelAdvancedForm.value.encoder,
            decoder: currentModelAdvancedForm.value.decoder,
            joiner: currentModelAdvancedForm.value.joiner,
            tokens: currentModelAdvancedForm.value.tokens,
        });
    }
}

async function saveConfig() {
    loading.value = true;

    try {
        // 保存高级配置到 Pinia
        saveAdvancedConfig();

        // 获取当前选中的完整模型配置
        const currentModel = settingsStore.currentModel;
        if (!currentModel) {
            throw new Error("请先选择一个模型");
        }

        // 同步到后端（包括音频配置）
        await syncModelToBackend();
        
        // 调试：输出当前音频配置
        console.log('[Audio] Saved audio config:', {
            audioSourceType: settingsStore.audioSourceType,
            audioDeviceIdForSystem: settingsStore.audioDeviceIdForSystem,
            audioDeviceIdForMicrophone: settingsStore.audioDeviceIdForMicrophone,
            currentDeviceId: currentAudioDeviceId.value
        });

        message.success("保存成功！");
    } catch (e) {
        message.error("保存失败: " + e);
        console.error("Failed to save config:", e);
    } finally {
        loading.value = false;
    }
}

// 导出设置
async function exportSettings() {
    try {
        const docDir = await documentDir();
        const filePath = await save({
            defaultPath: `${docDir}/live-subtitles-settings.json`,
            filters: [{ name: "JSON", extensions: ["json"] }],
        });
        if (filePath) {
            const jsonStr = settingsStore.exportSettings();
            await writeTextFile(filePath, jsonStr);
            message.success("导出成功！");
        }
    } catch (e) {
        message.error("导出失败: " + e);
        console.error("Failed to export settings:", e);
    }
}

// 导入设置
async function importSettings() {
    try {
        const filePath = await open({
            filters: [{ name: "JSON", extensions: ["json"] }],
        });
        if (filePath) {
            const jsonStr = await readTextFile(filePath);
            const result = settingsStore.importSettings(jsonStr);
            if (result.success) {
                message.success(result.message);
            } else {
                message.error(result.message);
            }
        }
    } catch (e) {
        message.error("导入失败: " + e);
        console.error("Failed to import settings:", e);
    }
}

// 重置设置
async function resetSettings() {
    const { Modal } = await import('ant-design-vue');
    Modal.confirm({
        title: '确认重置',
        content: '确定要将所有设置恢复为默认值吗？此操作不可撤销。',
        okText: '确认',
        cancelText: '取消',
        okType: 'danger',
        onOk() {
            settingsStore.resetToDefaults();
            message.success("已重置为默认值");
        },
    });
}

// 获取应用版本
async function fetchAppVersion() {
    try {
        const { getVersion } = await import('@tauri-apps/api/app');
        appVersion.value = await getVersion();
    } catch (e) {
        console.error('Failed to get app version:', e);
        appVersion.value = '0.1.0';
    }
}

// 检查更新（使用 Tauri updater 插件）
// 构建代理配置
function buildProxyConfig() {
    const config = {
        timeout: 60000 // 60秒超时，适应代理环境
    };

    // 如果启用了自定义代理
    if (settingsStore.useCustomProxy && settingsStore.proxyUrl) {
        config.proxy = {
            all: {
                url: settingsStore.proxyUrl
            }
        };

        // 如果配置了用户名和密码
        if (settingsStore.proxyUsername && settingsStore.proxyPassword) {
            config.proxy.all.basicAuth = {
                username: settingsStore.proxyUsername,
                password: settingsStore.proxyPassword
            };
        }
    }
    // 否则使用系统代理（默认行为）

    return config;
}

async function checkForUpdates() {
    checkingUpdate.value = true;
    hasUpdate.value = false;
    latestVersion.value = '';
    updateNotes.value = '';

    try {
        const { check } = await import('@tauri-apps/plugin-updater');
        const config = buildProxyConfig();
        const update = await check(config);

        if (update) {
            hasUpdate.value = update.available;
            if (update.available) {
                latestVersion.value = update.version;
                updateNotes.value = update.body || '';
                message.success(`发现新版本: ${update.version}`);
            } else {
                message.success('已是最新版本');
            }
        }
    } catch (e) {
        console.error('Failed to check for updates:', e);
        message.error('检查更新失败: ' + e.message);
    } finally {
        checkingUpdate.value = false;
    }
}

// 下载并安装更新
async function downloadAndInstallUpdate() {
    downloading.value = true;
    downloadProgress.value = 0;

    try {
        const { check } = await import('@tauri-apps/plugin-updater');
        const { relaunch } = await import('@tauri-apps/plugin-process');

        const config = buildProxyConfig();
        const update = await check(config);

        if (update?.available) {
            message.loading('正在下载更新...', 0);

            // 监听下载进度
            await update.downloadAndInstall((event) => {
                switch (event.event) {
                    case 'Started':
                        downloadProgress.value = 0;
                        break;
                    case 'Progress':
                        downloadProgress.value = Math.floor((event.data.downloaded / event.data.contentLength) * 100);
                        break;
                    case 'Finished':
                        downloadProgress.value = 100;
                        break;
                }
            });

            message.destroy();
            message.success('更新下载完成，即将重启应用...');

            // 延迟 1 秒后重启
            setTimeout(async () => {
                await relaunch();
            }, 1000);
        } else {
            message.info('没有可用更新');
        }
    } catch (e) {
        console.error('Failed to download update:', e);
        message.error('更新失败: ' + e.message);
    } finally {
        downloading.value = false;
        downloadProgress.value = 0;
    }
}

// 比较版本号 (a > b 返回 1, a < b 返回 -1, a == b 返回 0)
function compareVersions(a, b) {
    const aParts = a.split('.').map(Number);
    const bParts = b.split('.').map(Number);

    for (let i = 0; i < Math.max(aParts.length, bParts.length); i++) {
        const aVal = aParts[i] || 0;
        const bVal = bParts[i] || 0;

        if (aVal > bVal) return 1;
        if (aVal < bVal) return -1;
    }

    return 0;
}

// 显示关于对话框
async function showAbout() {
    aboutVisible.value = true;
    // 自动检查更新
    await checkForUpdates();
}

// 处理窗口状态记忆开关变化
async function handleWindowStateChange(checked) {
    if (checked) {
        // 启用时保存当前状态
        try {
            await saveWindowState(StateFlags.ALL);
        } catch (e) {
            console.error("Failed to save window state:", e);
        }
    }
    // 禁用时不需要特别处理，只是下次启动不会恢复
}

// 枚举音频设备
async function enumerateAudioDevices() {
    loadingAudioDevices.value = true;
    try {
        const devices = await invoke("enumerate_audio_devices");
        settingsStore.setAvailableAudioDevices(devices);
        message.success(`已检测到 ${devices.length} 个音频设备`);
    } catch (e) {
        message.error(`枚举音频设备失败: ${e}`);
        console.error("Failed to enumerate audio devices:", e);
    } finally {
        loadingAudioDevices.value = false;
    }
}

// 计算当前麦克风设备ID (支持双向绑定)
const currentAudioDeviceId = computed({
    get() {
        return settingsStore.audioDeviceIdForMicrophone;
    },
    set(value) {
        settingsStore.audioDeviceIdForMicrophone = value;
    }
});

// 计算过滤后的音频设备（根据音频源类型）
const filteredAudioDevices = computed(() => {
    const sourceType = settingsStore.audioSourceType;
    return settingsStore.availableAudioDevices.filter(device => {
        if (sourceType === 'systemaudio') {
            return device.device_type === 'output';
        } else if (sourceType === 'microphone') {
            return device.device_type === 'input';
        }
        return true;
    });
});

// 音频源类型变化时，自动切换到对应的设备ID并同步到后端
watch(() => settingsStore.audioSourceType, async (newType, oldType) => {
    console.log('[Audio] Audio source type changed to:', newType);
    console.log('[Audio] Switched to device ID:', currentAudioDeviceId.value);
    
    // 只有在实际切换时才同步（排除初始加载）
    if (oldType !== undefined && newType !== oldType) {
        // 检查识别是否正在运行
        let wasRunning = false;
        try {
            wasRunning = await invoke("is_recognition_running");
        } catch (e) {
            console.warn("Failed to check recognition status:", e);
        }
        
        // 如果正在运行，先停止
        if (wasRunning) {
            try {
                await invoke("stop_recognition");
                await new Promise(resolve => setTimeout(resolve, 500));
            } catch (e) {
                console.error("Failed to stop recognition:", e);
            }
        }
        
        // 同步音频配置到后端
        await syncModelToBackend();
        await new Promise(resolve => setTimeout(resolve, 300));
        
        // 如果之前在运行，重新启动
        if (wasRunning) {
            try {
                await invoke("start_recognition");
                message.success(`已切换到${newType === 'systemaudio' ? '系统音频' : '麦克风'}模式并重启识别`);
            } catch (e) {
                console.error("Failed to restart recognition:", e);
                message.warning(`已切换到${newType === 'systemaudio' ? '系统音频' : '麦克风'}模式，但重启失败: ${e}。请手动点击开始按钮。`);
            }
        } else {
            message.success(`已切换到${newType === 'systemaudio' ? '系统音频' : '麦克风'}模式`);
        }
    }
});

// 监听设备ID变化，自动同步到后端
watch(() => currentAudioDeviceId.value, async (newDeviceId, oldDeviceId) => {
    // 只有在设备ID实际发生变化时才同步（排除初始加载和undefined）
    if (oldDeviceId !== undefined && newDeviceId !== oldDeviceId) {
        console.log('[Audio] Device ID changed from', oldDeviceId, 'to', newDeviceId);
        
        // 检查识别是否正在运行
        let wasRunning = false;
        try {
            wasRunning = await invoke("is_recognition_running");
        } catch (e) {
            console.warn("Failed to check recognition status:", e);
        }
        
        // 如果正在运行，先停止
        if (wasRunning) {
            try {
                await invoke("stop_recognition");
                await new Promise(resolve => setTimeout(resolve, 500));
            } catch (e) {
                console.error("Failed to stop recognition:", e);
            }
        }
        
        // 同步配置
        await syncModelToBackend();
        await new Promise(resolve => setTimeout(resolve, 300));
        
        const deviceName = filteredAudioDevices.value.find(d => d.id === newDeviceId)?.name || '默认设备';
        
        // 如果之前在运行，重新启动
        if (wasRunning) {
            try {
                await invoke("start_recognition");
                message.success(`已切换到设备: ${deviceName}，识别已重启`);
            } catch (e) {
                console.error("Failed to restart recognition:", e);
                message.warning(`已切换到设备: ${deviceName}，但重启失败: ${e}。请手动点击开始按钮。`);
            }
        } else {
            message.success(`已切换到设备: ${deviceName}`);
        }
    }
});

onMounted(async () => {
    loadConfig();
    await fetchAppVersion();
    
    // 调试：显示从 localStorage 加载的音频配置
    console.log('[Audio] Loaded audio config from store:', {
        audioSourceType: settingsStore.audioSourceType,
        audioDeviceIdForSystem: settingsStore.audioDeviceIdForSystem,
        audioDeviceIdForMicrophone: settingsStore.audioDeviceIdForMicrophone
    });
    
    // 自动枚举音频设备
    await enumerateAudioDevices();
    
    // 枚举后再次检查当前设备ID是否有效
    if (currentAudioDeviceId.value) {
        const deviceExists = filteredAudioDevices.value.some(d => d.id === currentAudioDeviceId.value);
        console.log('[Audio] Device exists in filtered list:', deviceExists);
        
        if (!deviceExists) {
            console.warn('[Audio] Saved device not found, clearing selection');
            message.warning('之前保存的音频设备未找到，已重置为默认设备');
            currentAudioDeviceId.value = '';
        }
    }
});
</script>

<template>
    <a-config-provider :theme="{ algorithm: themeAlgorithm }">
        <div class="settings-wrapper" :class="{ 'dark-mode': isDark }">
            <!-- 自定义标题栏 -->
            <div class="title-bar" @mousedown="startDrag">
                <span class="title">设置</span>
                <div class="window-controls" @mousedown.stop>
                    <button class="control-btn" @click="minimizeWindow" title="最小化">
                        <MinusOutlined />
                    </button>
                    <button class="control-btn close-btn" @click="closeWindow" title="关闭">
                        <CloseOutlined />
                    </button>
                </div>
            </div>

            <div class="settings-container">

                <a-card title="模型配置" class="section-card">
                    <template #extra>
                        <a-typography-text type="secondary">配置 ASR 语音识别模型</a-typography-text>
                    </template>

                    <!-- 模型下载提示 -->
                    <a-alert type="info" show-icon style="margin-bottom: 16px;">
                        <template #message>
                            <div>模型下载地址：</div>
                            <div style="margin-top: 4px;">
                                <a href="https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html"
                                    target="_blank" style="color: #1890ff; margin-right: 16px;">sherpa-onnx 模型列表</a>
                                <a href="https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models" target="_blank"
                                    style="color: #1890ff;">GitHub Releases</a>
                            </div>
                        </template>
                    </a-alert>

                    <a-form layout="horizontal" class="aligned-form">
                        <!-- 模型根目录 -->
                        <div class="form-item-with-hint">
                            <a-form-item label="模型根目录">
                                <a-input-group compact class="full-width-input-group">
                                    <a-input v-model:value="settingsStore.modelsRootDir" placeholder="选择模型根目录路径" />
                                    <a-button @click="selectModelsRootDir">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                    <a-button @click="scanModelsRootDir" :loading="scanning" title="重新扫描">
                                        <template #icon>
                                            <ScanOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    设置模型根目录，该目录下的每个子文件夹将被识别为一个模型
                                </a-typography-text>
                            </div>
                        </div>

                        <!-- 模型选择 -->
                        <div class="form-item-with-hint">
                            <a-form-item label="选择模型">
                                <a-select v-model:value="settingsStore.currentModelId" style="width: 100%"
                                    placeholder="请先扫描模型目录"
                                    @change="(value) => console.log('Select changed to:', value, 'Store value:', settingsStore.currentModelId)"
                                    :options="settingsStore.availableModels.map(m => ({
                                        value: m.id,
                                        label: m.model_name + (m.is_complete ? ' ✓' : ' (不完整)') + (m.has_multiple_variants ? ' 📦' : '')
                                    }))" />
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    已扫描到 {{ settingsStore.availableModels.length }} 个模型，
                                    其中 {{settingsStore.availableModels.filter(m => m.is_complete).length}} 个完整可用
                                    <span v-if="currentModelDetails?.has_multiple_variants"> · 📦 此模型有多个版本可选</span>
                                </a-typography-text>
                            </div>
                        </div>

                        <!-- 模型版本选择（如果有多个版本） -->
                        <div v-if="currentModelDetails?.has_multiple_variants && currentModelDetails.variants?.length > 0"
                            class="form-item-with-hint">
                            <a-form-item label="模型版本">
                                <a-radio-group v-model:value="selectedVariant" button-style="solid"
                                    @change="() => switchModelVariant(selectedVariant)">
                                    <a-radio-button v-for="variant in currentModelDetails.variants"
                                        :key="variant.variant_name" :value="variant.variant_name">
                                        {{ variant.variant_name.toUpperCase() }}
                                        <span v-if="variant.variant_name === 'int8'"
                                            style="font-size: 10px; opacity: 0.7;">
                                            (快速)</span>
                                        <span v-if="variant.variant_name === 'fp32'"
                                            style="font-size: 10px; opacity: 0.7;">
                                            (精确)</span>
                                    </a-radio-button>
                                </a-radio-group>
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    int8: 量化版本，速度快，体积小 · fp32: 完整精度版本，识别更准确但较慢
                                </a-typography-text>
                            </div>
                        </div>

                        <!-- 当前模型配置（可编辑） -->
                        <div v-if="currentModelDetails" class="model-config-section">
                            <a-divider orientation="left" orientation-margin="0">
                                <span class="divider-title">模型文件配置</span>
                            </a-divider>

                            <a-form-item label="模型目录">
                                <a-input :value="currentModelDetails.model_dir" disabled />
                            </a-form-item>

                            <a-form-item label="Encoder">
                                <a-input-group compact class="full-width-input-group">
                                    <a-tooltip :title="currentModelAdvancedForm.encoder || '未配置'" placement="top">
                                        <a-input :value="getDisplayFileName(currentModelAdvancedForm.encoder)"
                                            @update:value="v => currentModelAdvancedForm.encoder = v.includes('\\') || v.includes('/') ? v : (modelDirPrefix + v)"
                                            placeholder="encoder.onnx"
                                            :status="currentModelAdvancedForm.encoder ? '' : 'error'" />
                                    </a-tooltip>
                                    <a-tooltip :title="currentModelAdvancedForm.encoder ? '已配置' : '未找到'">
                                        <a-button :type="currentModelAdvancedForm.encoder ? 'default' : 'default'"
                                            :danger="!currentModelAdvancedForm.encoder">
                                            <template #icon>
                                                <CheckCircleOutlined v-if="currentModelAdvancedForm.encoder"
                                                    style="color: #52c41a" />
                                                <CloseCircleOutlined v-else style="color: #ff4d4f" />
                                            </template>
                                        </a-button>
                                    </a-tooltip>
                                    <a-button @click="selectFile('encoder')" title="选择文件">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>

                            <a-form-item label="Decoder">
                                <a-input-group compact class="full-width-input-group">
                                    <a-tooltip :title="currentModelAdvancedForm.decoder || '未配置'" placement="top">
                                        <a-input :value="getDisplayFileName(currentModelAdvancedForm.decoder)"
                                            @update:value="v => currentModelAdvancedForm.decoder = v.includes('\\') || v.includes('/') ? v : (modelDirPrefix + v)"
                                            placeholder="decoder.onnx"
                                            :status="currentModelAdvancedForm.decoder ? '' : 'error'" />
                                    </a-tooltip>
                                    <a-tooltip :title="currentModelAdvancedForm.decoder ? '已配置' : '未找到'">
                                        <a-button :type="currentModelAdvancedForm.decoder ? 'default' : 'default'"
                                            :danger="!currentModelAdvancedForm.decoder">
                                            <template #icon>
                                                <CheckCircleOutlined v-if="currentModelAdvancedForm.decoder"
                                                    style="color: #52c41a" />
                                                <CloseCircleOutlined v-else style="color: #ff4d4f" />
                                            </template>
                                        </a-button>
                                    </a-tooltip>
                                    <a-button @click="selectFile('decoder')" title="选择文件">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>

                            <a-form-item label="Joiner">
                                <a-input-group compact class="full-width-input-group">
                                    <a-tooltip :title="currentModelAdvancedForm.joiner || '未配置'" placement="top">
                                        <a-input :value="getDisplayFileName(currentModelAdvancedForm.joiner)"
                                            @update:value="v => currentModelAdvancedForm.joiner = v.includes('\\') || v.includes('/') ? v : (modelDirPrefix + v)"
                                            placeholder="joiner.onnx"
                                            :status="currentModelAdvancedForm.joiner ? '' : 'error'" />
                                    </a-tooltip>
                                    <a-tooltip :title="currentModelAdvancedForm.joiner ? '已配置' : '未找到'">
                                        <a-button :type="currentModelAdvancedForm.joiner ? 'default' : 'default'"
                                            :danger="!currentModelAdvancedForm.joiner">
                                            <template #icon>
                                                <CheckCircleOutlined v-if="currentModelAdvancedForm.joiner"
                                                    style="color: #52c41a" />
                                                <CloseCircleOutlined v-else style="color: #ff4d4f" />
                                            </template>
                                        </a-button>
                                    </a-tooltip>
                                    <a-button @click="selectFile('joiner')" title="选择文件">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>

                            <a-form-item label="Tokens">
                                <a-input-group compact class="full-width-input-group">
                                    <a-tooltip :title="currentModelAdvancedForm.tokens || '未配置'" placement="top">
                                        <a-input :value="getDisplayFileName(currentModelAdvancedForm.tokens)"
                                            @update:value="v => currentModelAdvancedForm.tokens = v.includes('\\') || v.includes('/') ? v : (modelDirPrefix + v)"
                                            placeholder="tokens.txt"
                                            :status="currentModelAdvancedForm.tokens ? '' : 'error'" />
                                    </a-tooltip>
                                    <a-tooltip :title="currentModelAdvancedForm.tokens ? '已配置' : '未找到'">
                                        <a-button :type="currentModelAdvancedForm.tokens ? 'default' : 'default'"
                                            :danger="!currentModelAdvancedForm.tokens">
                                            <template #icon>
                                                <CheckCircleOutlined v-if="currentModelAdvancedForm.tokens"
                                                    style="color: #52c41a" />
                                                <CloseCircleOutlined v-else style="color: #ff4d4f" />
                                            </template>
                                        </a-button>
                                    </a-tooltip>
                                    <a-button @click="selectFile('tokens')" title="选择文件">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>

                            <div class="model-status-summary">
                                <a-alert :type="isModelComplete ? 'success' : 'warning'"
                                    :message="isModelComplete ? '模型配置完整，可以使用' : '模型配置不完整，请检查缺失的文件'" show-icon />
                            </div>
                        </div>
                    </a-form>
                </a-card>

                <a-card title="显示设置" class="section-card">
                    <template #extra>
                        <a-typography-text type="secondary">配置字幕显示相关选项</a-typography-text>
                    </template>

                    <a-form layout="horizontal" class="aligned-form">
                        <div class="form-item-with-hint">
                            <a-form-item label="显示历史字幕">
                                <a-switch v-model:checked="settingsStore.showHistory" />
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    启用后将在当前字幕上方显示历史识别内容
                                </a-typography-text>
                            </div>
                        </div>

                        <div class="form-item-with-hint">
                            <a-form-item label="历史最大长度">
                                <div class="inline-control">
                                    <a-input-number v-model:value="settingsStore.maxHistoryLength" :min="0"
                                        placeholder="0 表示无限制" style="width: 150px" />
                                    <span class="input-suffix">字符</span>
                                </div>
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    设置历史字幕文本的最大显示长度，0 表示无限制
                                </a-typography-text>
                            </div>
                        </div>
                    </a-form>
                </a-card>

                <a-card title="音频配置" class="section-card">
                    <template #extra>
                        <a-typography-text type="secondary">配置音频输入源</a-typography-text>
                    </template>

                    <a-form layout="horizontal" class="aligned-form">
                        <!-- 音频源类型 -->
                        <div class="form-item-with-hint">
                            <a-form-item label="音频源类型">
                                <a-radio-group v-model:value="settingsStore.audioSourceType" 
                                    :options="audioSourceTypeOptions"
                                    option-type="button" button-style="solid" />
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    选择系统音频可识别电脑播放的声音，选择麦克风输入可识别语音
                                </a-typography-text>
                            </div>
                        </div>

                        <!-- 麦克风设备选择（仅在麦克风模式显示） -->
                        <div class="form-item-with-hint" v-if="settingsStore.audioSourceType === 'microphone'">
                            <a-form-item label="麦克风设备">
                                <a-input-group compact class="full-width-input-group">
                                    <a-select v-model:value="currentAudioDeviceId" 
                                        style="width: calc(100% - 40px)"
                                        placeholder="选择麦克风设备（留空使用默认设备）"
                                        allow-clear
                                        show-search
                                        @change="(value) => console.log('[Audio] Device selection changed to:', value)"
                                        :filter-option="(input, option) => {
                                            return option.label.toLowerCase().indexOf(input.toLowerCase()) >= 0;
                                        }">
                                        <a-select-option v-for="device in filteredAudioDevices" 
                                            :key="device.id" 
                                            :value="device.id"
                                            :label="device.name">
                                            <span>
                                                <CheckCircleOutlined v-if="device.is_default" 
                                                    style="color: #52c41a; margin-right: 4px;" />
                                                {{ device.name }}
                                            </span>
                                        </a-select-option>
                                    </a-select>
                                    <a-button @click="enumerateAudioDevices" 
                                        :loading="loadingAudioDevices" 
                                        title="刷新麦克风列表">
                                        <template #icon>
                                            <ReloadOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    已检测到 {{ filteredAudioDevices.length }} 个麦克风设备
                                    <CheckCircleOutlined style="color: #52c41a; margin: 0 4px;" />
                                    标记表示默认设备
                                </a-typography-text>
                            </div>
                        </div>
                    </a-form>
                </a-card>

                <a-card title="网络设置" class="section-card">
                    <template #extra>
                        <a-typography-text type="secondary">配置网络代理选项</a-typography-text>
                    </template>

                    <a-form layout="horizontal" class="aligned-form">
                        <div class="form-item-with-hint">
                            <a-form-item label="使用自定义代理">
                                <a-switch v-model:checked="settingsStore.useCustomProxy" />
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    启用后将使用自定义代理地址进行网络请求（检查更新、下载安装包等）
                                </a-typography-text>
                            </div>
                        </div>

                        <a-form-item label="代理地址" v-if="settingsStore.useCustomProxy">
                            <a-input v-model:value="settingsStore.proxyUrl"
                                placeholder="例如: http://proxy.example.com:8080" style="width: 100%" />
                        </a-form-item>

                        <div v-if="settingsStore.useCustomProxy">
                            <a-divider orientation="left">
                                <span style="font-size: 13px; color: rgba(0, 0, 0, 0.45);">代理认证（可选）</span>
                            </a-divider>

                            <a-form-item label="用户名">
                                <a-input v-model:value="settingsStore.proxyUsername" placeholder="代理服务器用户名（如需认证）"
                                    style="width: 100%" />
                            </a-form-item>

                            <a-form-item label="密码">
                                <a-input-password v-model:value="settingsStore.proxyPassword"
                                    placeholder="代理服务器密码（如需认证）" style="width: 100%" />
                            </a-form-item>
                        </div>
                    </a-form>
                </a-card>

                <a-card title="外观设置" class="section-card">
                    <template #extra>
                        <a-typography-text type="secondary">配置应用外观主题</a-typography-text>
                    </template>

                    <a-form layout="horizontal" class="aligned-form">
                        <a-form-item label="主题模式">
                            <a-radio-group v-model:value="settingsStore.themeMode" :options="themeOptions"
                                option-type="button" button-style="solid" />
                        </a-form-item>

                        <div class="form-item-with-hint">
                            <a-form-item label="记住窗口状态">
                                <a-switch v-model:checked="settingsStore.rememberWindowState"
                                    @change="handleWindowStateChange" />
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    启用后将在下次启动时恢复窗口位置和大小
                                </a-typography-text>
                            </div>
                        </div>
                    </a-form>
                </a-card>

                <!-- 底部留白，防止被固定按钮遮挡 -->
                <div style="height: 80px;"></div>

                <!-- 固定在底部的保存按钮 -->
                <a-affix :offset-bottom="24">
                    <div class="save-affix">
                        <a-button type="primary" :loading="loading" @click="saveConfig">
                            <template #icon>
                                <SaveOutlined />
                            </template>
                            {{ loading ? "保存中..." : "保存" }}
                        </a-button>
                        <a-button @click="exportSettings">
                            <template #icon>
                                <DownloadOutlined />
                            </template>
                            导出配置
                        </a-button>
                        <a-button @click="importSettings">
                            <template #icon>
                                <UploadOutlined />
                            </template>
                            导入配置
                        </a-button>
                        <a-button danger @click="resetSettings">
                            <template #icon>
                                <ReloadOutlined />
                            </template>
                            重置
                        </a-button>
                        <a-button @click="showAbout">
                            <template #icon>
                                <InfoCircleOutlined />
                            </template>
                            关于
                        </a-button>
                    </div>
                </a-affix>
            </div>
        </div>

        <!-- 关于对话框 -->
        <a-modal v-model:open="aboutVisible" title="Live Subtitles" :width="800">
            <a-descriptions :column="1" bordered size="small">
                <a-descriptions-item label="当前版本">
                    <a-space>
                        <span>{{ appVersion }}</span>
                        <a-tag v-if="hasUpdate" color="red">有新版本</a-tag>
                    </a-space>
                </a-descriptions-item>
                <a-descriptions-item label="最新版本" v-if="latestVersion">
                    <a-space>
                        <span>{{ latestVersion }}</span>
                        <a href="https://github.com/caolib/live-subtitles/releases" target="_blank">
                            查看发布
                        </a>
                    </a-space>
                </a-descriptions-item>
                <a-descriptions-item label="介绍">
                    基于 Tauri v2 的实时字幕桌面应用，使用 sherpa-onnx 进行语音识别。
                </a-descriptions-item>
                <a-descriptions-item label="仓库地址">
                    <a href="https://github.com/caolib/live-subtitles" target="_blank">
                        https://github.com/caolib/live-subtitles
                    </a>
                </a-descriptions-item>
                <a-descriptions-item label="许可证">MIT License</a-descriptions-item>
            </a-descriptions>

            <!-- 更新通知 -->
            <a-alert v-if="hasUpdate && updateNotes" type="info" style="margin-top: 16px;" show-icon>
                <template #message>
                    <div style="font-weight: 500;">新版本更新内容</div>
                </template>
                <template #description>
                    <div style="white-space: pre-wrap; max-height: 200px; overflow-y: auto;">{{ updateNotes }}</div>
                </template>
            </a-alert>

            <!-- 下载进度 -->
            <a-progress v-if="downloading" :percent="downloadProgress" status="active" style="margin-top: 16px;" />

            <a-divider />

            <div>
                <h4 style="margin-bottom: 12px;">致谢</h4>
                <a-space direction="vertical" style="width: 100%;">
                    <div>
                        <a href="https://github.com/k2-fsa/sherpa-onnx" target="_blank">
                            sherpa-onnx
                        </a>
                        <span style="color: #8c8c8c;"> - 提供语音识别模型</span>
                    </div>
                    <div>
                        <a href="https://github.com/jxlpzqc/TMSpeech" target="_blank">
                            TMSpeech
                        </a>
                        <span style="color: #8c8c8c;"> - 灵感来源</span>
                    </div>
                </a-space>
            </div>

            <template #footer>
                <a-space>
                    <a-button v-if="hasUpdate" type="primary" :loading="downloading" @click="downloadAndInstallUpdate">
                        <template #icon>
                            <DownloadOutlined />
                        </template>
                        立即更新
                    </a-button>
                    <a-button :loading="checkingUpdate" @click="checkForUpdates">
                        <template #icon>
                            <SyncOutlined />
                        </template>
                        检查更新
                    </a-button>
                    <a-button @click="aboutVisible = false">关闭</a-button>
                </a-space>
            </template>
        </a-modal>
    </a-config-provider>
</template>

<style>
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body,
div,
span,
input,
button,
select,
textarea {
    font-family: var(--main-font-family) !important;
}

/* 全局滚动条样式 */
::-webkit-scrollbar {
    width: 4px;
    height: 8px;
}

::-webkit-scrollbar-track {
    background: transparent;
}

::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.3);
}

/* 深色模式滚动条 */
.dark-mode ::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
}

.dark-mode ::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
}
</style>

<style scoped>
.settings-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    transition: background-color 0.3s;
}

.settings-wrapper.dark-mode {
    background-color: #141414;
}

/* 自定义标题栏 */
.title-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    height: 40px;
    padding: 0 12px;
    background: #f5f5f5;
    border-bottom: 1px solid #e8e8e8;
    user-select: none;
    cursor: move;
    flex-shrink: 0;
}

.dark-mode .title-bar {
    background: #1f1f1f;
    border-bottom-color: #303030;
}

.title-bar .title {
    font-size: 14px;
    font-weight: 500;
    color: #333;
}

.dark-mode .title-bar .title {
    color: #fff;
}

.window-controls {
    display: flex;
    gap: 8px;
}

.control-btn {
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: #666;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
}

.dark-mode .control-btn {
    color: #aaa;
}

.control-btn:hover {
    background: rgba(0, 0, 0, 0.1);
}

.dark-mode .control-btn:hover {
    background: rgba(255, 255, 255, 0.1);
}

.control-btn.close-btn:hover {
    background: #e81123;
    color: #fff;
}

.settings-container {
    flex: 1;
    overflow-y: auto;
    margin: 0 auto;
    padding: 24px;
    width: 100%;
}


/* 表单对齐样式 */
.aligned-form :deep(.ant-form-item) {
    align-items: flex-start;
    margin-bottom: 16px;
}

.aligned-form :deep(.ant-form-item-label) {
    width: 120px;
    flex-shrink: 0;
    text-align: left;
    padding-right: 12px;
}

.aligned-form :deep(.ant-form-item-label > label) {
    height: auto;
    line-height: 32px;
}

.form-item-content {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
}

.form-item-with-hint {
    margin-bottom: 16px;
}

.form-item-with-hint :deep(.ant-form-item) {
    margin-bottom: 4px;
}

.inline-control {
    display: flex;
    align-items: center;
}

.field-hint {
    display: block;
    font-size: 12px;
    line-height: 1.4;
    text-align: left;
}

.input-suffix {
    margin-left: 8px;
    color: rgba(0, 0, 0, 0.45);
}

.dark-mode .input-suffix {
    color: rgba(255, 255, 255, 0.45);
}

/* 模型配置区域 */
.model-config-section {
    margin-top: 16px;
    padding-top: 8px;
}

.model-config-section .divider-title {
    font-size: 13px;
    color: rgba(0, 0, 0, 0.65);
}

.dark-mode .model-config-section .divider-title {
    color: rgba(255, 255, 255, 0.65);
}

.model-status-summary {
    margin-top: 16px;
}

/* 输入框组全宽 */
.full-width-input-group {
    display: flex !important;
    width: 100% !important;
}

.full-width-input-group :deep(.ant-input) {
    flex: 1 !important;
    min-width: 0;
}

/* 确保 form-item-control 占满宽度 */
.aligned-form :deep(.ant-form-item-control) {
    flex: 1;
    max-width: calc(100% - 132px);
}

.aligned-form :deep(.ant-input-group) {
    display: flex !important;
    width: 100% !important;
}

.aligned-form :deep(.ant-input-group .ant-input) {
    flex: 1 !important;
}

.save-affix {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    background: rgba(255, 255, 255, 0.95);
    backdrop-filter: blur(8px);
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    width: fit-content;
}

.dark-mode .save-affix {
    background: rgba(30, 30, 30, 0.95);
}
</style>
