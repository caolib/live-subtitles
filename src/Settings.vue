<script setup>
import { ref, onMounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { documentDir } from "@tauri-apps/api/path";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveWindowState, StateFlags } from "@tauri-apps/plugin-window-state";
import { theme } from "ant-design-vue";
import {
    FolderOpenOutlined,
    DownloadOutlined,
    UploadOutlined,
    ReloadOutlined,
    SaveOutlined,
    MinusOutlined,
    CloseOutlined,
    ScanOutlined
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

// 配置状态
const config = ref({
    current_model_id: "",
    models: [],
});

// 模型目录
const modelsDir = ref("");

// 模型配置表单
const modelForm = ref({
    modelDir: "",    // 模型文件夹路径
    encoder: "",
    decoder: "",
    joiner: "",
    tokens: "",
});

// 是否展开高级设置
const showAdvanced = ref([]);

const loading = ref(false);
const saveStatus = ref("");

// 加载配置
async function loadConfig() {
    try {
        // 获取模型目录
        modelsDir.value = await invoke("get_models_dir");

        const cfg = await invoke("get_config");
        config.value = cfg;

        // 加载当前模型的路径
        const currentModel = cfg.models.find((m) => m.id === cfg.current_model_id);
        if (currentModel && currentModel.model_type.type === "Transducer") {
            modelForm.value = {
                modelDir: currentModel.model_dir || "",
                encoder: currentModel.model_type.encoder,
                decoder: currentModel.model_type.decoder,
                joiner: currentModel.model_type.joiner,
                tokens: currentModel.tokens,
            };
        }
    } catch (e) {
        console.error("Failed to load config:", e);
    }
}

// 选择文件
async function selectFile(field) {
    try {
        const selected = await open({
            multiple: false,
            defaultPath: modelForm.value.modelDir || modelsDir.value,
            filters: [
                {
                    name: field === "tokens" ? "Tokens" : "ONNX Model",
                    extensions: field === "tokens" ? ["txt"] : ["onnx"],
                },
            ],
        });
        if (selected) {
            modelForm.value[field] = selected;
        }
    } catch (e) {
        console.error("Failed to select file:", e);
    }
}

// 选择模型文件夹
async function selectModelDir() {
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            defaultPath: modelsDir.value,
        });
        if (selected) {
            modelForm.value.modelDir = selected;
            // 自动扫描模型文件
            await scanModelFiles();
        }
    } catch (e) {
        console.error("Failed to select directory:", e);
    }
}

// 扫描模型文件
async function scanModelFiles() {
    if (!modelForm.value.modelDir) {
        saveStatus.value = "请先选择模型文件夹";
        setTimeout(() => { saveStatus.value = ""; }, 2000);
        return;
    }

    try {
        const result = await invoke("scan_model_dir", { dirPath: modelForm.value.modelDir });

        // 自动填充扫描到的文件
        if (result.encoder) modelForm.value.encoder = result.encoder;
        if (result.decoder) modelForm.value.decoder = result.decoder;
        if (result.joiner) modelForm.value.joiner = result.joiner;
        if (result.tokens) modelForm.value.tokens = result.tokens;

        // 检查是否完整
        const missing = [];
        if (!result.encoder) missing.push("encoder");
        if (!result.decoder) missing.push("decoder");
        if (!result.joiner) missing.push("joiner");
        if (!result.tokens) missing.push("tokens");

        if (missing.length === 0) {
            saveStatus.value = `已识别模型: ${result.model_name}`;
        } else {
            saveStatus.value = `部分文件未找到: ${missing.join(", ")}，请手动设置`;
            showAdvanced.value = ['advanced'];
        }
        setTimeout(() => { saveStatus.value = ""; }, 3000);
    } catch (e) {
        saveStatus.value = "扫描失败: " + e;
        console.error("Failed to scan model dir:", e);
        setTimeout(() => { saveStatus.value = ""; }, 2000);
    }
}

onMounted(() => {
    loadConfig();
});
async function saveConfig() {
    loading.value = true;
    saveStatus.value = "";

    try {
        // 更新模型配置
        const updatedConfig = {
            ...config.value,
            models: config.value.models.map((m) => {
                if (m.id === config.value.current_model_id) {
                    return {
                        ...m,
                        model_dir: modelForm.value.modelDir,
                        model_type: {
                            type: "Transducer",
                            encoder: modelForm.value.encoder,
                            decoder: modelForm.value.decoder,
                            joiner: modelForm.value.joiner,
                        },
                        tokens: modelForm.value.tokens,
                    };
                }
                return m;
            }),
        };

        await invoke("update_config", { config: updatedConfig });

        // Pinia store 会自动持久化，无需手动保存

        saveStatus.value = "保存成功！";

        setTimeout(() => {
            saveStatus.value = "";
        }, 2000);
    } catch (e) {
        saveStatus.value = "保存失败: " + e;
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
            saveStatus.value = "导出成功！";
            setTimeout(() => { saveStatus.value = ""; }, 2000);
        }
    } catch (e) {
        saveStatus.value = "导出失败: " + e;
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
            saveStatus.value = result.message;
            setTimeout(() => { saveStatus.value = ""; }, 2000);
        }
    } catch (e) {
        saveStatus.value = "导入失败: " + e;
        console.error("Failed to import settings:", e);
    }
}

// 重置设置
function resetSettings() {
    settingsStore.resetToDefaults();
    saveStatus.value = "已重置为默认值";
    setTimeout(() => { saveStatus.value = ""; }, 2000);
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

onMounted(() => {
    loadConfig();
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

                    <a-form layout="horizontal" class="aligned-form">
                        <div class="form-item-with-hint">
                            <a-form-item label="模型文件夹">
                                <a-input-group compact class="full-width-input-group">
                                    <a-input v-model:value="modelForm.modelDir" placeholder="选择模型文件夹路径" />
                                    <a-button @click="selectModelDir">
                                        <template #icon>
                                            <FolderOpenOutlined />
                                        </template>
                                    </a-button>
                                    <a-button @click="scanModelFiles" title="重新扫描">
                                        <template #icon>
                                            <ScanOutlined />
                                        </template>
                                    </a-button>
                                </a-input-group>
                            </a-form-item>
                            <div class="full-width-hint">
                                <a-typography-text type="secondary" class="field-hint">
                                    选择包含模型文件的文件夹，将自动识别 encoder、decoder、joiner 和 tokens 文件
                                </a-typography-text>
                            </div>
                        </div>

                        <!-- 高级设置折叠面板 -->
                        <a-collapse v-model:activeKey="showAdvanced" ghost class="advanced-collapse">
                            <a-collapse-panel key="advanced" header="高级设置（手动配置模型文件路径）">
                                <a-form-item label="Encoder">
                                    <a-input-group compact class="full-width-input-group">
                                        <a-input v-model:value="modelForm.encoder" placeholder="encoder.onnx" />
                                        <a-button @click="selectFile('encoder')">
                                            <template #icon>
                                                <FolderOpenOutlined />
                                            </template>
                                        </a-button>
                                    </a-input-group>
                                </a-form-item>

                                <a-form-item label="Decoder">
                                    <a-input-group compact class="full-width-input-group">
                                        <a-input v-model:value="modelForm.decoder" placeholder="decoder.onnx" />
                                        <a-button @click="selectFile('decoder')">
                                            <template #icon>
                                                <FolderOpenOutlined />
                                            </template>
                                        </a-button>
                                    </a-input-group>
                                </a-form-item>

                                <a-form-item label="Joiner">
                                    <a-input-group compact class="full-width-input-group">
                                        <a-input v-model:value="modelForm.joiner" placeholder="joiner.onnx" />
                                        <a-button @click="selectFile('joiner')">
                                            <template #icon>
                                                <FolderOpenOutlined />
                                            </template>
                                        </a-button>
                                    </a-input-group>
                                </a-form-item>

                                <a-form-item label="Tokens">
                                    <a-input-group compact class="full-width-input-group">
                                        <a-input v-model:value="modelForm.tokens" placeholder="tokens.txt" />
                                        <a-button @click="selectFile('tokens')">
                                            <template #icon>
                                                <FolderOpenOutlined />
                                            </template>
                                        </a-button>
                                    </a-input-group>
                                </a-form-item>
                            </a-collapse-panel>
                        </a-collapse>
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
                        <a-typography-text v-if="saveStatus" :type="saveStatus.includes('成功') ? 'success' : 'danger'">
                            {{ saveStatus }}
                        </a-typography-text>
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
                    </div>
                </a-affix>
            </div>
        </div>
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

/* 高级设置折叠面板 */
.advanced-collapse {
    margin-top: 8px;
}

.advanced-collapse :deep(.ant-collapse-header) {
    padding: 8px 0 !important;
    color: #1890ff !important;
}

.advanced-collapse :deep(.ant-collapse-content-box) {
    padding: 16px 0 0 0 !important;
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
