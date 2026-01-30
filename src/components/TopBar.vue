<script setup>
import {
    CaretRightOutlined,
    PauseOutlined,
    CopyOutlined,
    DeleteOutlined,
    MinusOutlined,
    CloseOutlined,
    LockOutlined,
    UnlockOutlined,
    MessageOutlined,
    CommentOutlined,
    FormatPainterOutlined,
    BorderOutlined,
    FullscreenExitOutlined,
    FontSizeOutlined,
    SettingOutlined
} from "@ant-design/icons-vue";

const props = defineProps({
    isLocked: Boolean,
    isRunning: Boolean,
    showHistory: Boolean,
    lowercaseSubtitle: Boolean,
    isMaximized: Boolean,
    styleObj: Object
});

const emit = defineEmits([
    'toggleLock',
    'toggleRecognition',
    'toggleHistory',
    'toggleCase',
    'copyAllText',
    'clearSubtitles',
    'openStyleEditor',
    'openSettings',
    'minimizeWindow',
    'toggleMaximize',
    'hideWindow',
    'startDrag'
]);
</script>

<template>
    <div class="top-bar" :class="{ locked: isLocked }" :style="styleObj" @mousedown="emit('startDrag')">
        <!-- 锁定时只显示解锁按钮 -->
        <template v-if="isLocked">
            <div class="top-bar-center" @mousedown.stop>
                <button class="action-btn unlock-btn" @click="emit('toggleLock')" title="解锁窗口">
                    <UnlockOutlined />
                </button>
            </div>
        </template>

        <!-- 未锁定时显示所有按钮 -->
        <template v-else>
            <div class="top-bar-left" @mousedown.stop>
                <button class="action-btn" :class="{ active: isRunning }" @click="emit('toggleRecognition')"
                    :title="isRunning ? '停止识别' : '开始识别'">
                    <PauseOutlined v-if="isRunning" />
                    <CaretRightOutlined v-else />
                </button>
                <button class="action-btn" :class="{ active: showHistory }" @click="emit('toggleHistory')"
                    :title="showHistory ? '隐藏历史' : '显示历史'">
                    <MessageOutlined v-if="showHistory" />
                    <CommentOutlined v-else />
                </button>
                <button class="action-btn" :class="{ active: lowercaseSubtitle }" @click="emit('toggleCase')"
                    :title="lowercaseSubtitle ? '当前:小写' : '当前:原始'">
                    <FontSizeOutlined />
                </button>
                <button class="action-btn" @click="emit('copyAllText')" title="复制全部">
                    <CopyOutlined />
                </button>
                <button class="action-btn" @click="emit('clearSubtitles')" title="清空字幕">
                    <DeleteOutlined />
                </button>
                <button class="action-btn" @click="emit('toggleLock')" title="锁定窗口">
                    <LockOutlined />
                </button>
                <button class="action-btn" @click="emit('openStyleEditor')" title="编辑样式">
                    <FormatPainterOutlined />
                </button>
                <button class="action-btn" @click="emit('openSettings')" title="设置">
                    <SettingOutlined />
                </button>
            </div>
            <div class="top-bar-right" @mousedown.stop>
                <button class="control-btn" @click="emit('minimizeWindow')" title="最小化">
                    <MinusOutlined />
                </button>
                <button class="control-btn" @click="emit('toggleMaximize')" :title="isMaximized ? '还原' : '最大化'">
                    <FullscreenExitOutlined v-if="isMaximized" />
                    <BorderOutlined v-else />
                </button>
                <button class="control-btn close-btn" @click="emit('hideWindow')" title="隐藏到托盘">
                    <CloseOutlined />
                </button>
            </div>
        </template>
    </div>
</template>
