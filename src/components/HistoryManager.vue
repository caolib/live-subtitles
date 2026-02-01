<template>
    <div class="history-manager">
        <!-- 搜索栏 -->
        <div class="history-search">
            <a-input v-model:value="searchKeyword" placeholder="搜索历史记录..." allow-clear @input="debouncedSearch" />
        </div>

        <!-- 工具栏 -->
        <div class="history-toolbar">
            <a-space>
                <a-button size="small" @click="refreshList">
                    <template #icon>
                        <ReloadOutlined />
                    </template>
                    刷新
                </a-button>
                <a-button size="small" @click="openHistoryDir">
                    <template #icon>
                        <FolderOpenOutlined />
                    </template>
                    打开目录
                </a-button>
                <a-popconfirm title="确定要清空所有历史记录吗？" ok-text="确定" cancel-text="取消" @confirm="clearAllHistory">
                    <a-button size="small" danger :disabled="historyList.length === 0">
                        <template #icon>
                            <DeleteOutlined />
                        </template>
                        清空全部
                    </a-button>
                </a-popconfirm>
            </a-space>
            <span class="history-count">共 {{ historyList.length }} 条记录</span>
        </div>

        <!-- 历史记录列表 -->
        <div class="history-list">
            <a-spin :spinning="loading">
                <a-empty v-if="historyList.length === 0 && !loading" description="暂无历史记录" />
                <div v-else class="history-items">
                    <div v-for="item in historyList" :key="item.id" class="history-item" @click="showDetail(item)">
                        <div class="history-item-header">
                            <span class="history-date">{{ formatDate(item.start_time) }}</span>
                            <span class="history-duration">{{ formatDuration(item.duration_secs) }}</span>
                        </div>
                        <div class="history-preview">{{ item.preview || '(无内容)' }}</div>
                        <div class="history-item-footer">
                            <span class="history-record-count">{{ item.record_count }} 条字幕</span>
                            <a-space class="history-actions" @click.stop>
                                <a-button type="text" size="small" @click="showDetail(item)">
                                    <template #icon>
                                        <EyeOutlined />
                                    </template>
                                </a-button>
                                <a-popconfirm title="确定删除这条记录吗？" ok-text="确定" cancel-text="取消"
                                    @confirm="deleteHistory(item.id)">
                                    <a-button type="text" size="small" danger>
                                        <template #icon>
                                            <DeleteOutlined />
                                        </template>
                                    </a-button>
                                </a-popconfirm>
                            </a-space>
                        </div>
                    </div>
                </div>
            </a-spin>
        </div>

        <!-- 详情弹窗 -->
        <a-modal v-model:open="detailVisible" :title="detailTitle" width="600px" :footer="null">
            <div class="history-detail">
                <div class="detail-meta">
                    <p><strong>开始时间:</strong> {{ detailSession?.start_time ? formatDateTime(detailSession.start_time) :
                        '-' }}
                    </p>
                    <p><strong>结束时间:</strong> {{ detailSession?.end_time ? formatDateTime(detailSession.end_time) : '-'
                        }}</p>
                    <p><strong>时长:</strong> {{ detailSession ? formatDuration((detailSession.end_time -
                        detailSession.start_time) / 1000) : '-' }}</p>
                    <p><strong>字幕条数:</strong> {{ detailSession?.records?.length || 0 }}</p>
                </div>
                <a-divider />
                <div class="detail-content">
                    <div class="detail-toolbar">
                        <a-button size="small" @click="copyAllText">
                            <template #icon>
                                <CopyOutlined />
                            </template>
                            复制全部文本
                        </a-button>
                    </div>
                    <div class="subtitle-records">
                        <div v-for="(record, index) in detailSession?.records || []" :key="index"
                            class="subtitle-record">
                            <span class="record-time">{{ formatTime(record.timestamp) }}</span>
                            <span class="record-text">{{ record.text }}</span>
                        </div>
                    </div>
                </div>
            </div>
        </a-modal>
    </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { message } from 'ant-design-vue'
import {
    ReloadOutlined,
    FolderOpenOutlined,
    DeleteOutlined,
    EyeOutlined,
    CopyOutlined
} from '@ant-design/icons-vue'

const historyList = ref([])
const loading = ref(false)
const searchKeyword = ref('')
const detailVisible = ref(false)
const detailSession = ref(null)
const detailTitle = ref('历史记录详情')

// 防抖搜索相关
let searchTimer = null
const DEBOUNCE_DELAY = 300 // 防抖延迟 300ms

// 防抖搜索函数
function debouncedSearch() {
    if (searchTimer) {
        clearTimeout(searchTimer)
    }
    searchTimer = setTimeout(() => {
        handleSearch(searchKeyword.value)
    }, DEBOUNCE_DELAY)
}

// 加载历史记录列表
async function loadHistoryList() {
    loading.value = true
    try {
        const list = await invoke('get_history_list')
        historyList.value = list
    } catch (e) {
        console.error('加载历史记录失败:', e)
        message.error('加载历史记录失败: ' + e)
    } finally {
        loading.value = false
    }
}

// 刷新列表
async function refreshList() {
    if (searchKeyword.value) {
        await handleSearch(searchKeyword.value)
    } else {
        await loadHistoryList()
    }
}

// 搜索历史记录
async function handleSearch(keyword) {
    if (!keyword || !keyword.trim()) {
        await loadHistoryList()
        return
    }

    loading.value = true
    try {
        const list = await invoke('search_history', { keyword: keyword.trim() })
        historyList.value = list
    } catch (e) {
        console.error('搜索历史记录失败:', e)
        message.error('搜索失败: ' + e)
    } finally {
        loading.value = false
    }
}

// 组件卸载时清理定时器
onUnmounted(() => {
    if (searchTimer) {
        clearTimeout(searchTimer)
    }
})

// 显示详情
async function showDetail(item) {
    try {
        const session = await invoke('get_history_detail', { id: item.id })
        detailSession.value = session
        detailTitle.value = `历史记录 - ${formatDate(item.start_time)}`
        detailVisible.value = true
    } catch (e) {
        console.error('获取历史记录详情失败:', e)
        message.error('获取详情失败: ' + e)
    }
}

// 删除历史记录
async function deleteHistory(id) {
    try {
        await invoke('delete_history', { id })
        message.success('删除成功')
        await refreshList()
    } catch (e) {
        console.error('删除历史记录失败:', e)
        message.error('删除失败: ' + e)
    }
}

// 清空所有历史记录
async function clearAllHistory() {
    try {
        const count = await invoke('clear_all_history')
        message.success(`已清空 ${count} 条历史记录`)
        historyList.value = []
    } catch (e) {
        console.error('清空历史记录失败:', e)
        message.error('清空失败: ' + e)
    }
}

// 打开历史记录目录
async function openHistoryDir() {
    try {
        await invoke('open_history_dir')
    } catch (e) {
        console.error('打开目录失败:', e)
        message.error('打开目录失败: ' + e)
    }
}

// 复制全部文本
async function copyAllText() {
    if (!detailSession.value?.records?.length) {
        message.warning('没有可复制的内容')
        return
    }

    const text = detailSession.value.records
        .map(r => r.text)
        .join('\n')

    try {
        await navigator.clipboard.writeText(text)
        message.success('已复制到剪贴板')
    } catch (e) {
        console.error('复制失败:', e)
        message.error('复制失败')
    }
}

// 格式化日期（只显示日期部分）
function formatDate(timestamp) {
    const date = new Date(timestamp)
    return date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
    })
}

// 格式化完整日期时间
function formatDateTime(timestamp) {
    const date = new Date(timestamp)
    return date.toLocaleString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    })
}

// 格式化时间（只显示时间部分）
function formatTime(timestamp) {
    const date = new Date(timestamp)
    return date.toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
    })
}

// 格式化时长
function formatDuration(seconds) {
    if (!seconds || seconds < 0) return '0秒'

    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    const secs = Math.floor(seconds % 60)

    if (hours > 0) {
        return `${hours}时${minutes}分${secs}秒`
    } else if (minutes > 0) {
        return `${minutes}分${secs}秒`
    } else {
        return `${secs}秒`
    }
}

onMounted(() => {
    loadHistoryList()
})

// 暴露刷新方法供父组件调用
defineExpose({
    refresh: refreshList
})
</script>

<style scoped>
.history-manager {
    display: flex;
    flex-direction: column;
    height: 100%;
}

.history-search {
    margin-bottom: 12px;
}

.history-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
}

.history-count {
    color: #888;
    font-size: 12px;
}

.history-list {
    flex: 1;
    overflow-y: auto;
    min-height: 200px;
    max-height: 400px;
}

.history-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.history-item {
    padding: 12px;
    border: 1px solid #474747;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
}

.history-item:hover {
    border-color: #1890ff;
    box-shadow: 0 2px 8px rgba(24, 144, 255, 0.15);
}

.history-item-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}

.history-date {
    color: #828282;
}

.history-duration {
    color: #888;
    font-size: 12px;
}

.history-preview {
    font-size: 13px;
    line-height: 1.5;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    margin-bottom: 8px;
}

.history-item-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.history-record-count {
    color: #888;
    font-size: 12px;
}

.history-actions {
    opacity: 0;
    transition: opacity 0.2s;
}

.history-item:hover .history-actions {
    opacity: 1;
}

/* 详情弹窗样式 */
.history-detail {
    max-height: 60vh;
    overflow-y: auto;
}

.detail-meta {
    padding: 12px;
    border-radius: 6px;
    border: 1px solid var(--border-color, #e8e8e8);
}

.detail-meta p {
    margin: 4px 0;
    font-size: 13px;
}

.detail-content {
    margin-top: 12px;
}

.detail-toolbar {
    margin-bottom: 12px;
}

.subtitle-records {
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid var(--border-color, #e8e8e8);
    border-radius: 6px;
    padding: 8px;
}

.subtitle-record {
    display: flex;
    gap: 12px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border-color, #f0f0f0);
}

.subtitle-record:last-child {
    border-bottom: none;
}

.record-time {
    color: var(--text-color-secondary, #888);
    font-size: 12px;
    font-family: monospace;
    flex-shrink: 0;
}

.record-text {
    font-size: 13px;
    line-height: 1.5;
}
</style>
