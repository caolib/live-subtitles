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

    // ========== 后续可添加更多设置 ==========
    // 例如：
    // const fontSize = ref(20)
    // const fontFamily = ref('default')
    // const opacity = ref(0.9)

    // ========== 计算属性 ==========
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
        // 后续添加更多分类
        // audio: { ... }
        // recognition: { ... }
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

            // 后续添加更多分类的导入
            // if (data.settings.audio) { ... }

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
        // 后续添加更多默认值重置
    }

    return {
        // 状态
        showHistory,
        maxHistoryLength,
        themeMode,
        rememberWindowState,

        // 计算属性
        exportableSettings,

        // Actions
        updateDisplaySettings,
        updateAppearanceSettings,
        updateWindowSettings,
        exportSettings,
        importSettings,
        resetToDefaults,
    }
}, {
    // Pinia 持久化配置
    persist: {
        key: 'live-subtitles-settings',
        storage: localStorage,
        // 只持久化需要的字段
        pick: ['showHistory', 'maxHistoryLength', 'themeMode', 'rememberWindowState'],
    },
})
