<script setup>
import { LoadingOutlined } from "@ant-design/icons-vue";

const props = defineProps({
    styleObj: Object,
    historyText: String,
    historySubtitleStyle: Object,
    latestSubtitle: String,
    currentSubtitleStyle: Object,
    isRunning: Boolean,
    isModelLoading: Boolean,
    errorMessage: String,
    isHovering: Boolean,
    currentModelName: String
});
</script>

<template>
    <div class="subtitle-area" :style="styleObj">
        <!-- 历史字幕（合并显示，可滚动） -->
        <div class="history-text" v-if="historyText" :style="historySubtitleStyle">
            {{ historyText }}
        </div>

        <!-- 当前字幕（固定在底部） -->
        <div class="current-subtitle" v-if="latestSubtitle" :style="currentSubtitleStyle">
            {{ latestSubtitle }}
        </div>

        <!-- 空状态 -->
        <div class="empty-state" v-else-if="!isRunning">
            <span>点击开始按钮开始识别</span>
        </div>
        <div class="loading-state" v-else-if="isModelLoading">
            <LoadingOutlined class="loading-icon" spin />
            <span>正在加载模型，请稍候...</span>
        </div>
        <div class="empty-state" v-else>
            <span>正在聆听...</span>
        </div>

        <!-- 错误提示 -->
        <div class="error-message" v-if="errorMessage">
            {{ errorMessage }}
        </div>

        <!-- 模型名称显示（右下角，悬停时显示） -->
        <div class="model-name" v-show="isHovering" @mousedown.stop>
            {{ currentModelName }}
        </div>
    </div>
</template>
