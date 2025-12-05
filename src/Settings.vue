<script setup>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { documentDir } from "@tauri-apps/api/path";
import { FolderOpened, Check, Download, Upload, RefreshRight } from "@element-plus/icons-vue";
import { useSettingsStore } from "./stores/settings";

// Pinia Store
const settingsStore = useSettingsStore();

// 配置状态
const config = ref({
    current_model_id: "",
    models: [],
});

// 模型目录
const modelsDir = ref("");

// 模型配置表单
const modelForm = ref({
    encoder: "",
    decoder: "",
    joiner: "",
    tokens: "",
});

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
            defaultPath: modelsDir.value,
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

onMounted(() => {
    loadConfig();
});
</script>

<template>
    <div class="settings-container">
        <h1>设置</h1>

        <div class="section">
            <h2>模型配置</h2>
            <p class="section-desc">配置 ASR 语音识别模型文件路径</p>

            <div class="form-group">
                <label>Encoder 模型</label>
                <div class="file-input">
                    <input type="text" v-model="modelForm.encoder" placeholder="encoder.onnx" />
                    <button @click="selectFile('encoder')" title="选择文件">
                        <FolderOpened />
                    </button>
                </div>
            </div>

            <div class="form-group">
                <label>Decoder 模型</label>
                <div class="file-input">
                    <input type="text" v-model="modelForm.decoder" placeholder="decoder.onnx" />
                    <button @click="selectFile('decoder')" title="选择文件">
                        <FolderOpened />
                    </button>
                </div>
            </div>

            <div class="form-group">
                <label>Joiner 模型</label>
                <div class="file-input">
                    <input type="text" v-model="modelForm.joiner" placeholder="joiner.onnx" />
                    <button @click="selectFile('joiner')" title="选择文件">
                        <FolderOpened />
                    </button>
                </div>
            </div>

            <div class="form-group">
                <label>Tokens 文件</label>
                <div class="file-input">
                    <input type="text" v-model="modelForm.tokens" placeholder="tokens.txt" />
                    <button @click="selectFile('tokens')" title="选择文件">
                        <FolderOpened />
                    </button>
                </div>
            </div>
        </div>

        <div class="section">
            <h2>显示设置</h2>
            <p class="section-desc">配置字幕显示相关选项</p>

            <div class="form-group">
                <label class="checkbox-label">
                    <input type="checkbox" v-model="settingsStore.showHistory" />
                    <span>显示历史字幕</span>
                </label>
                <p class="field-desc">启用后将在当前字幕上方显示历史识别内容</p>
            </div>

            <div class="form-group">
                <label>历史字幕最大长度</label>
                <div class="number-input">
                    <input type="number" v-model.number="settingsStore.maxHistoryLength" min="0"
                        placeholder="0 表示无限制" />
                    <span class="input-hint">字符</span>
                </div>
                <p class="field-desc">设置历史字幕文本的最大显示长度，0 表示无限制</p>
            </div>
        </div>

        <div class="section">
            <h2>配置管理</h2>
            <p class="section-desc">导入、导出或重置配置</p>

            <div class="config-actions">
                <button class="config-btn" @click="exportSettings" title="导出配置">
                    <Download />
                    <span>导出配置</span>
                </button>
                <button class="config-btn" @click="importSettings" title="导入配置">
                    <Upload />
                    <span>导入配置</span>
                </button>
                <button class="config-btn reset-btn" @click="resetSettings" title="重置为默认值">
                    <RefreshRight />
                    <span>重置</span>
                </button>
            </div>
        </div>

        <div class="actions">
            <button class="save-btn" @click="saveConfig" :disabled="loading">
                <Check v-if="!loading" />
                {{ loading ? "保存中..." : "保存模型配置" }}
            </button>
            <span class="save-status" :class="{ success: saveStatus.includes('成功') }">
                {{ saveStatus }}
            </span>
        </div>
    </div>
</template>

<style>
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    background: #1e1e1e;
    color: #e0e0e0;
    min-height: 100vh;
}

body,
input,
button,
select,
textarea {
    font-family: var(--main-font-family);
}
</style>

<style scoped>
.settings-container {
    margin: 0 auto;
    padding: 24px;
}

h1 {
    font-size: 24px;
    font-weight: 500;
    margin-bottom: 24px;
    color: #fff;
}

.section {
    background: #2d2d2d;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
}

h2 {
    font-size: 16px;
    font-weight: 500;
    margin-bottom: 8px;
    color: #fff;
}

.section-desc {
    font-size: 13px;
    color: #888;
    margin-bottom: 16px;
}

.form-group {
    margin-bottom: 16px;
}

.form-group label {
    display: block;
    font-size: 13px;
    color: #aaa;
    margin-bottom: 6px;
}

.file-input {
    display: flex;
    gap: 8px;
}

.file-input input {
    flex: 1;
    padding: 8px 12px;
    background: #1e1e1e;
    border: 1px solid #444;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 13px;
}

.file-input input:focus {
    outline: none;
    border-color: #0078d4;
}

.file-input button {
    padding: 8px 12px;
    background: #3d3d3d;
    border: 1px solid #444;
    border-radius: 4px;
    color: #e0e0e0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
}

.file-input button:hover {
    background: #4d4d4d;
}

.file-input button svg {
    width: 16px;
    height: 16px;
}

.actions {
    display: flex;
    align-items: center;
    gap: 12px;
}

.save-btn {
    padding: 10px 20px;
    background: #0078d4;
    border: none;
    border-radius: 4px;
    color: #fff;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
}

.save-btn:hover {
    background: #1084d8;
}

.save-btn:disabled {
    background: #555;
    cursor: not-allowed;
}

.save-btn svg {
    width: 16px;
    height: 16px;
}

.save-status {
    font-size: 13px;
    color: #f44;
}

.save-status.success {
    color: #4caf50;
}

/* 复选框样式 */
.checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 14px;
    color: #e0e0e0;
}

.checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
    accent-color: #0078d4;
}

/* 数字输入框样式 */
.number-input {
    display: flex;
    align-items: center;
    gap: 8px;
}

.number-input input {
    width: 120px;
    padding: 8px 12px;
    background: #1e1e1e;
    border: 1px solid #444;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 13px;
}

.number-input input:focus {
    outline: none;
    border-color: #0078d4;
}

.input-hint {
    font-size: 13px;
    color: #888;
}

.field-desc {
    font-size: 12px;
    color: #666;
    margin-top: 4px;
}

/* 配置管理按钮 */
.config-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
}

.config-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: #3d3d3d;
    border: 1px solid #444;
    border-radius: 4px;
    color: #e0e0e0;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.2s;
}

.config-btn:hover {
    background: #4d4d4d;
}

.config-btn svg {
    width: 16px;
    height: 16px;
}

.config-btn.reset-btn {
    border-color: #d32f2f;
    color: #ff6b6b;
}

.config-btn.reset-btn:hover {
    background: rgba(211, 47, 47, 0.2);
}
</style>
