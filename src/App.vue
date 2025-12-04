<script setup>
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  VideoPlay,
  VideoPause,
  DocumentCopy,
  Delete,
  Minus,
  Close
} from "@element-plus/icons-vue";

// 状态
const isRunning = ref(false);
const subtitles = ref([]); // 已完成的字幕历史
const currentText = ref(""); // 正在识别的文本（中间结果）
const maxSubtitles = 5; // 最多显示的字幕条数
const errorMessage = ref("");

// 拖动相关
const appWindow = getCurrentWindow();

// 开始/停止识别
async function toggleRecognition() {
  try {
    if (isRunning.value) {
      await invoke("stop_recognition");
      isRunning.value = false;
    } else {
      await invoke("start_recognition");
      isRunning.value = true;
      errorMessage.value = "";
    }
  } catch (e) {
    errorMessage.value = String(e);
    console.error("Recognition error:", e);
  }
}

// 清空字幕
function clearSubtitles() {
  subtitles.value = [];
  currentText.value = "";
}

// 复制所有文本
async function copyAllText() {
  const allText = [...subtitles.value.map(s => s.text), currentText.value]
    .filter(t => t && t.trim())
    .join('\n');
  if (allText) {
    try {
      await navigator.clipboard.writeText(allText);
    } catch (e) {
      console.error('Failed to copy:', e);
    }
  }
}

// 关闭窗口
async function closeWindow() {
  if (isRunning.value) {
    await invoke("stop_recognition");
  }
  await appWindow.close();
}

// 最小化窗口
async function minimizeWindow() {
  await appWindow.minimize();
}

// 开始拖动
function startDrag() {
  appWindow.startDragging();
}

// 监听字幕事件
let unlistenSubtitle = null;
let unlistenError = null;

onMounted(async () => {
  // 检查初始状态
  try {
    isRunning.value = await invoke("is_recognition_running");
  } catch (e) {
    console.error("Failed to get initial state:", e);
  }

  // 监听字幕事件
  unlistenSubtitle = await listen("subtitle", (event) => {
    const subtitle = event.payload;
    if (subtitle.text && subtitle.text.trim()) {
      if (subtitle.is_final) {
        // 句子结束，添加到历史记录
        subtitles.value.push({
          id: Date.now(),
          text: subtitle.text,
          timestamp: subtitle.timestamp,
        });
        // 保持最大条数
        if (subtitles.value.length > maxSubtitles) {
          subtitles.value.shift();
        }
        // 清空当前正在识别的文本
        currentText.value = "";
      } else {
        // 中间结果，更新当前正在识别的文本（替换而不是追加）
        currentText.value = subtitle.text;
      }
    }
  });

  // 监听错误事件
  unlistenError = await listen("recognition_error", (event) => {
    errorMessage.value = String(event.payload);
    isRunning.value = false;
  });

  // 自动开始识别
  if (!isRunning.value) {
    try {
      await invoke("start_recognition");
      isRunning.value = true;
    } catch (e) {
      errorMessage.value = String(e);
      console.error("Auto start failed:", e);
    }
  }
});

onUnmounted(() => {
  if (unlistenSubtitle) unlistenSubtitle();
  if (unlistenError) unlistenError();
});

// 最新的字幕（正在识别的文本，或最后一条已完成的）
const latestSubtitle = computed(() => {
  if (currentText.value) {
    return currentText.value;
  }
  if (subtitles.value.length === 0) return "";
  return subtitles.value[subtitles.value.length - 1].text;
});

// 历史字幕 (已完成的字幕，如果有正在识别的则显示全部，否则除了最后一条)
const historySubtitles = computed(() => {
  if (currentText.value) {
    // 有正在识别的文本，显示所有已完成的字幕
    return subtitles.value;
  }
  // 没有正在识别的文本，最后一条作为最新字幕显示
  if (subtitles.value.length <= 1) return [];
  return subtitles.value.slice(0, -1);
});
</script>

<template>
  <div class="app-container">
    <!-- 顶部控制栏（自动隐藏） -->
    <div class="top-bar" @mousedown="startDrag">
      <div class="top-bar-left" @mousedown.stop>
        <button class="action-btn" :class="{ active: isRunning }" @click="toggleRecognition"
          :title="isRunning ? '停止识别' : '开始识别'">
          <VideoPause v-if="isRunning" />
          <VideoPlay v-else />
        </button>
        <button class="action-btn" @click="copyAllText" title="复制全部">
          <DocumentCopy />
        </button>
        <button class="action-btn" @click="clearSubtitles" title="清空字幕">
          <Delete />
        </button>
      </div>
      <div class="top-bar-right" @mousedown.stop>
        <button class="control-btn" @click="minimizeWindow" title="最小化">
          <Minus />
        </button>
        <button class="control-btn close-btn" @click="closeWindow" title="关闭">
          <Close />
        </button>
      </div>
    </div>

    <!-- 字幕区域 -->
    <div class="subtitle-area">
      <!-- 历史字幕（合并显示，可滚动） -->
      <div class="history-text" v-if="historySubtitles.length > 0">
        {{historySubtitles.map(s => s.text).join(' ')}}
      </div>

      <!-- 当前字幕（固定在底部） -->
      <div class="current-subtitle" v-if="latestSubtitle">
        {{ latestSubtitle }}
      </div>

      <!-- 空状态 -->
      <div class="empty-state" v-else-if="!isRunning">
        <span>点击开始按钮开始识别</span>
      </div>
      <div class="empty-state" v-else>
        <span>正在聆听...</span>
      </div>

      <!-- 错误提示 -->
      <div class="error-message" v-if="errorMessage">
        {{ errorMessage }}
      </div>
    </div>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body,
#app {
  height: 100%;
  overflow: hidden;
  background: transparent;
}

body {
  background: transparent;
  font-family: 'JetBrains Mono', 'Cascadia Code', '汉仪有圆', '喵字果汁体', 'Microsoft YaHei', 'PingFang SC', 'Segoe UI', 'Helvetica Neue', Arial, sans-serif;
}
</style>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: rgba(30, 30, 30, 0.9);
  overflow: hidden;
  backdrop-filter: blur(10px);
}

/* 顶部控制栏（自动隐藏） */
.top-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background: rgba(0, 0, 0, 0.5);
  user-select: none;
  opacity: 0;
  transform: translateY(-100%);
  transition: opacity 0.3s ease, transform 0.3s ease;
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 100;
  border-radius: 16px 16px 0 0;
  cursor: move;
}

.app-container:hover .top-bar {
  opacity: 1;
  transform: translateY(0);
}

.top-bar-left {
  display: flex;
  gap: 6px;
}

.top-bar-right {
  display: flex;
  gap: 6px;
}

.control-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
}

.control-btn svg {
  width: 14px;
  height: 14px;
}

.control-btn:hover {
  background: rgba(255, 255, 255, 0.2);
}

.close-btn:hover {
  background: #e81123;
}

.action-btn {
  padding: 4px 8px;
  border: none;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.action-btn svg {
  width: 14px;
  height: 14px;
}

.action-btn:hover {
  background: rgba(255, 255, 255, 0.25);
}

.action-btn.active {
  background: #e81123;
}

.action-btn.active:hover {
  background: #c41019;
}

/* 字幕区域 */
.subtitle-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: 12px 16px;
  overflow: hidden;
  min-height: 0;
}

/* 历史字幕可滚动 */
.history-text {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.4);
  line-height: 1.5;
  margin-bottom: 8px;
  text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.5);
  overflow-y: auto;
  flex-shrink: 1;
  min-height: 0;
  max-height: 50%;
}

/* 自定义滚动条 */
.history-text::-webkit-scrollbar {
  width: 6px;
}

.history-text::-webkit-scrollbar-track {
  background: transparent;
}

.history-text::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.2);
  border-radius: 3px;
}

.history-text::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

.current-subtitle {
  font-size: 20px;
  color: #fff;
  font-weight: 500;
  line-height: 1.4;
  text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.5);
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(5px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.empty-state {
  color: rgba(255, 255, 255, 0.4);
  font-size: 14px;
  text-align: center;
  padding: 20px;
}

.error-message {
  color: #ff6b6b;
  font-size: 12px;
  margin-top: 8px;
  padding: 8px;
  background: rgba(255, 0, 0, 0.1);
  border-radius: 4px;
}
</style>
