<script setup>
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useSettingsStore } from "./stores/settings";

import { useWindowControl } from "./composables/useWindowControl";
import { useSubtitle } from "./composables/useSubtitle";
import { useStyleManager } from "./composables/useStyleManager";
import { useAppConfig } from "./composables/useAppConfig";
import { useContextMenu } from "./composables/useContextMenu";

// import TopBar from "./components/TopBar.vue";
import SubtitleDisplay from "./components/SubtitleDisplay.vue";
import { CloseOutlined, MinusOutlined, FullscreenOutlined, FullscreenExitOutlined, HolderOutlined } from '@ant-design/icons-vue';

// Pinia Store
const settingsStore = useSettingsStore();
const { currentModel } = storeToRefs(settingsStore);

// Composables
const {
  isLocked,
  isExpanded,
  isHovering,
  isMaximized,
  handleMouseEnter,
  handleMouseLeave,
  hideWindow,
  minimizeWindow,
  toggleMaximize,
  startDrag,
  toggleLock,
  startResize
} = useWindowControl(settingsStore);

const {
  isRunning,
  isModelLoading,
  errorMessage,
  latestSubtitle,
  historyText,
  toggleRecognition,
  clearSubtitles,
  copyAllText,
  toggleCase,
  toggleHistory
} = useSubtitle(settingsStore);

const {
  openStyleEditor
} = useStyleManager();

const {
  openSettings
} = useAppConfig(settingsStore);

// Context Menu (Right Click)
const windowControl = {
  isLocked, isExpanded, isHovering, isMaximized, handleMouseEnter, handleMouseLeave,
  hideWindow, minimizeWindow, toggleMaximize, startDrag, toggleLock, startResize
};
const subtitleControl = {
  isRunning, isModelLoading, errorMessage, latestSubtitle, historyText,
  toggleRecognition, clearSubtitles, copyAllText, toggleCase, toggleHistory
};
const styleManager = { openStyleEditor };
const appConfig = { openSettings };

const { showContextMenu } = useContextMenu({
  settingsStore,
  windowControl,
  subtitleControl,
  styleManager,
  appConfig
});

// Helper for UI
const currentModelName = computed(() => {
  const model = currentModel.value;
  return model?.model_name || '未配置模型';
});

// Hex 转 RGBA
function hexToRgba(hex, alpha) {
  let c;
  if (/^#([A-Fa-f0-9]{3}){1,2}$/.test(hex)) {
    c = hex.substring(1).split('');
    if (c.length === 3) {
      c = [c[0], c[0], c[1], c[1], c[2], c[2]];
    }
    c = '0x' + c.join('');
    return 'rgba(' + [(c >> 16) & 255, (c >> 8) & 255, c & 255].join(',') + ',' + alpha + ')';
  }
  return hex;
}

// Styles
const dynamicTopBarStyle = computed(() => ({
  backgroundColor: hexToRgba(settingsStore.subtitleBackgroundColor, settingsStore.subtitleBackgroundOpacity),
}));

const dynamicSubtitleAreaStyle = computed(() => ({
  backgroundColor: hexToRgba(settingsStore.subtitleBackgroundColor, settingsStore.subtitleBackgroundOpacity),
}));

const dynamicCurrentSubtitleStyle = computed(() => ({
  fontSize: settingsStore.subtitleFontSize + 'px',
  color: settingsStore.subtitleColor,
}));

const dynamicHistorySubtitleStyle = computed(() => ({
  fontSize: Math.max(12, settingsStore.subtitleFontSize * 0.7) + 'px',
  color: settingsStore.subtitleColor,
  opacity: 0.6
}));

</script>

<template>
  <div class="window-wrapper">
    <!-- 自定义调整大小手柄 (只有未锁定时可用) -->
    <div v-if="!isLocked" class="resize-handle top" @mousedown.prevent="startResize('North')"></div>
    <div v-if="!isLocked" class="resize-handle bottom" @mousedown.prevent="startResize('South')"></div>

    <div class="app-container" :class="{ expanded: isExpanded }" @mouseenter="handleMouseEnter"
      @mouseleave="handleMouseLeave" @contextmenu="showContextMenu" @mousedown="startDrag">

      <!-- 悬浮显示的窗口控制按钮 (右上角) -->
      <div class="window-controls" v-show="isHovering || isExpanded" @mousedown.stop>
        <!-- 拖拽手柄 -->
        <div class="control-btn drag-handle" @mousedown.stop="startDrag" title="拖动窗口">
          <HolderOutlined />
        </div>
        <div class="control-btn" @click="minimizeWindow" title="最小化">
          <MinusOutlined />
        </div>
        <div class="control-btn" @click="toggleMaximize" title="最大化/还原">
          <component :is="isMaximized ? FullscreenExitOutlined : FullscreenOutlined" />
        </div>
        <div class="control-btn close" @click="hideWindow" title="隐藏到托盘">
          <CloseOutlined />
        </div>
      </div>

      <SubtitleDisplay :styleObj="dynamicSubtitleAreaStyle" :historyText="historyText"
        :historySubtitleStyle="dynamicHistorySubtitleStyle" :latestSubtitle="latestSubtitle"
        :currentSubtitleStyle="dynamicCurrentSubtitleStyle" :isRunning="isRunning" :isModelLoading="isModelLoading"
        :errorMessage="errorMessage" :isHovering="isHovering" :currentModelName="currentModelName" />
    </div>
  </div>
</template>

<style scoped>
/* 整个窗口容器，用于定位 resize handles */
.window-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
}

/* 调整大小手柄区域 */
.resize-handle {
  position: absolute;
  left: 0;
  right: 0;
  height: 12px;
  /* 增加感应区域高度 */
  z-index: 9999;
  /* 确保在顶层 */
}

.resize-handle.top {
  top: 0;
  cursor: ns-resize;
}

.resize-handle.bottom {
  bottom: 0;
  cursor: ns-resize;
}

/* 窗口控制按钮 (右上角悬浮) */
.window-controls {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  align-items: center;
  gap: 0;
  z-index: 1000;
  pointer-events: auto;

  /* 统一的深色背景容器 */
  background: rgba(0, 0, 0, 0.4);
  border-bottom-left-radius: 6px;
  padding: 2px 4px;
  backdrop-filter: blur(4px);
}

.control-btn {
  width: 32px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  color: rgba(255, 255, 255, 0.9);
  background: transparent;
  /* 移除单个按钮的背景 */
  cursor: pointer;
  transition: all 0.2s;
  font-size: 14px;
}

.control-btn:hover {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.control-btn.drag-handle {
  cursor: grab;
}

.control-btn.drag-handle:active {
  cursor: grabbing;
}

.control-btn.close:hover {
  background: #ff4d4f;
  color: white;
}
</style>
