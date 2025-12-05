<script setup>
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpened, Check } from "@element-plus/icons-vue";

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
        if (currentModel && currentModel.model_type.Transducer) {
            modelForm.value = {
                encoder: currentModel.model_type.Transducer.encoder,
                decoder: currentModel.model_type.Transducer.decoder,
                joiner: currentModel.model_type.Transducer.joiner,
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

// 保存配置
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
                            Transducer: {
                                encoder: modelForm.value.encoder,
                                decoder: modelForm.value.decoder,
                                joiner: modelForm.value.joiner,
                            },
                        },
                        tokens: modelForm.value.tokens,
                    };
                }
                return m;
            }),
        };

        await invoke("update_config", { config: updatedConfig });
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

        <div class="actions">
            <button class="save-btn" @click="saveConfig" :disabled="loading">
                <Check v-if="!loading" />
                {{ loading ? "保存中..." : "保存设置" }}
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
    font-family: 'Segoe UI', 'Microsoft YaHei', sans-serif;
    background: #1e1e1e;
    color: #e0e0e0;
    min-height: 100vh;
}
</style>

<style scoped>
.settings-container {
    max-width: 600px;
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
</style>
