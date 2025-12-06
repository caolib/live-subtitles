import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

/**
 * 设置 Store
 * 用于管理应用的所有配置，支持持久化和导入导出
 */
export const useSettingsStore = defineStore('settings', () => {
    // ========== 显示设置 ==========
    const showHistory = ref(true)
    const maxHistoryLength = ref(0) // 0 表示无限制

    // ========== 主题设置 ==========
    // 'light' | 'dark' | 'system'
    const themeMode = ref('system')

    // ========== 窗口设置 ==========
    const rememberWindowState = ref(true) // 记住窗口位置和大小

    // ========== 模型设置 ==========
    const modelsRootDir = ref('') // 模型根目录
    const currentModelId = ref('') // 当前选中的模型 ID（目录名）
    const availableModels = ref([]) // 可用的模型列表 [{id, name, encoder, decoder, joiner, tokens, isComplete}]
    const modelAdvancedConfig = ref({}) // 高级配置：每个模型的手动覆盖配置 {modelId: {encoder, decoder, joiner, tokens}}

    // ========== 后续可添加更多设置 ==========
    // 例如：
    // const fontSize = ref(20)
    // const fontFamily = ref('default')
    // const opacity = ref(0.9)

    // ========== 计算属性 ==========
    // 获取当前选中的模型配置
    const currentModel = computed(() => {
        const model = availableModels.value.find(m => m.id === currentModelId.value)
        if (!model) return null

        // 如果有高级配置覆盖，合并配置
        const advancedConfig = modelAdvancedConfig.value[model.id]
        if (advancedConfig) {
            return {
                ...model,
                encoder: advancedConfig.encoder || model.encoder,
                decoder: advancedConfig.decoder || model.decoder,
                joiner: advancedConfig.joiner || model.joiner,
                tokens: advancedConfig.tokens || model.tokens,
            }
        }
        return model
    })

    // 获取所有可导出的设置
    const exportableSettings = computed(() => ({
        display: {
            showHistory: showHistory.value,
            maxHistoryLength: maxHistoryLength.value,
        },
        appearance: {
            themeMode: themeMode.value,
        },
        window: {
            rememberWindowState: rememberWindowState.value,
        },
        model: {
            modelsRootDir: modelsRootDir.value,
            currentModelId: currentModelId.value,
            modelAdvancedConfig: modelAdvancedConfig.value,
        },
    }))

    // ========== Actions ==========

    /**
     * 更新显示设置
     */
    function updateDisplaySettings(settings) {
        if (settings.showHistory !== undefined) {
            showHistory.value = settings.showHistory
        }
        if (settings.maxHistoryLength !== undefined) {
            maxHistoryLength.value = settings.maxHistoryLength
        }
    }

    /**
     * 更新外观设置
     */
    function updateAppearanceSettings(settings) {
        if (settings.themeMode !== undefined) {
            themeMode.value = settings.themeMode
        }
    }

    /**
     * 更新窗口设置
     */
    function updateWindowSettings(settings) {
        if (settings.rememberWindowState !== undefined) {
            rememberWindowState.value = settings.rememberWindowState
        }
    }

    /**
     * 更新模型设置
     */
    function updateModelSettings(settings) {
        if (settings.modelsRootDir !== undefined) {
            modelsRootDir.value = settings.modelsRootDir
        }
        if (settings.currentModelId !== undefined) {
            currentModelId.value = settings.currentModelId
        }
        if (settings.modelAdvancedConfig !== undefined) {
            modelAdvancedConfig.value = settings.modelAdvancedConfig
        }
    }

    /**
     * 设置可用模型列表（从扫描结果更新）
     */
    function setAvailableModels(models) {
        availableModels.value = models
        // 如果当前选中的模型不在列表中，自动选择第一个完整的模型
        if (!models.find(m => m.id === currentModelId.value)) {
            const firstComplete = models.find(m => m.isComplete)
            if (firstComplete) {
                currentModelId.value = firstComplete.id
            } else if (models.length > 0) {
                currentModelId.value = models[0].id
            } else {
                currentModelId.value = ''
            }
        }
    }

    /**
     * 设置当前模型的高级配置
     */
    function setModelAdvancedConfig(modelId, config) {
        modelAdvancedConfig.value = {
            ...modelAdvancedConfig.value,
            [modelId]: config
        }
    }

    /**
     * 获取当前模型（非响应式，用于手动调用）
     */
    function getCurrentModelSync() {
        console.log('getCurrentModelSync called, currentModelId:', currentModelId.value);
        console.log('Available models:', availableModels.value);

        const model = availableModels.value.find(m => m.id === currentModelId.value);
        console.log('Found model:', model);

        if (!model) return null;

        // 如果有高级配置覆盖，合并配置
        const advancedConfig = modelAdvancedConfig.value[model.id];
        if (advancedConfig) {
            return {
                ...model,
                encoder: advancedConfig.encoder || model.encoder,
                decoder: advancedConfig.decoder || model.decoder,
                joiner: advancedConfig.joiner || model.joiner,
                tokens: advancedConfig.tokens || model.tokens,
            };
        }
        return model;
    }

    /**
     * 导出所有设置为 JSON 字符串
     */
    function exportSettings() {
        const settings = {
            version: 1, // 配置版本号，用于后续兼容性处理
            exportedAt: new Date().toISOString(),
            settings: exportableSettings.value,
        }
        return JSON.stringify(settings, null, 2)
    }

    /**
     * 从 JSON 字符串导入设置
     */
    function importSettings(jsonString) {
        try {
            const data = JSON.parse(jsonString)

            // 版本兼容性检查
            if (!data.version || !data.settings) {
                throw new Error('无效的配置文件格式')
            }

            // 导入显示设置
            if (data.settings.display) {
                updateDisplaySettings(data.settings.display)
            }

            // 导入外观设置
            if (data.settings.appearance) {
                updateAppearanceSettings(data.settings.appearance)
            }

            // 导入窗口设置
            if (data.settings.window) {
                updateWindowSettings(data.settings.window)
            }

            // 导入模型设置
            if (data.settings.model) {
                updateModelSettings(data.settings.model)
            }

            return { success: true, message: '导入成功' }
        } catch (e) {
            return { success: false, message: `导入失败: ${e.message}` }
        }
    }

    /**
     * 重置所有设置为默认值
     */
    function resetToDefaults() {
        showHistory.value = true
        maxHistoryLength.value = 0
        themeMode.value = 'system'
        rememberWindowState.value = true
        modelsRootDir.value = ''
        currentModelId.value = ''
        availableModels.value = []
        modelAdvancedConfig.value = {}
    }

    return {
        // 状态
        showHistory,
        maxHistoryLength,
        themeMode,
        rememberWindowState,
        modelsRootDir,
        currentModelId,
        availableModels,
        modelAdvancedConfig,

        // 计算属性
        currentModel,
        exportableSettings,

        // Actions
        updateDisplaySettings,
        updateAppearanceSettings,
        updateWindowSettings,
        updateModelSettings,
        setAvailableModels,
        setModelAdvancedConfig,
        getCurrentModelSync,
        exportSettings,
        importSettings,
        resetToDefaults,
    }
}, {
    // Pinia 持久化配置
    persist: {
        key: 'live-subtitles-settings',
        storage: localStorage,
        // 持久化所有需要保存的字段
        pick: [
            'showHistory',
            'maxHistoryLength',
            'themeMode',
            'rememberWindowState',
            'modelsRootDir',
            'currentModelId',
            'availableModels',
            'modelAdvancedConfig'
        ],
    },
})
