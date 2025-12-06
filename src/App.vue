<script setup>
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { restoreStateCurrent, StateFlags } from "@tauri-apps/plugin-window-state";
import { readTextFile } from "@tauri-apps/plugin-fs";
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
  FormatPainterOutlined
} from "@ant-design/icons-vue";
import { Select } from "ant-design-vue";
import { useSettingsStore } from "./stores/settings";

// Pinia Store
const settingsStore = useSettingsStore();

// 状态
const isRunning = ref(false);
const isLocked = ref(false); // 窗口锁定状态
const subtitles = ref([]); // 已完成的字幕历史
const currentText = ref(""); // 正在识别的文本（中间结果）
const maxSubtitles = 5; // 最多显示的字幕条数
const errorMessage = ref("");
const isHovering = ref(false); // 鼠标是否在窗口上
const isSelectOpen = ref(false); // 下拉列表是否打开

// 当前模型名称 - 现在直接从 store computed，因为是同一个 Vue 实例
const currentModelName = computed(() => {
  return settingsStore.currentModel?.model_name || '未配置模型';
});

// 可用模型列表
const modelOptions = computed(() => {
  return settingsStore.availableModels.map(m => ({
    label: m.model_name + (m.is_complete ? ' ✓' : ' (不完整)'),
    value: m.id,
    disabled: !m.is_complete
  }));
});

// 切换模型
async function switchModel(modelId) {
  if (modelId === settingsStore.currentModelId) return;

  console.log(`Switching to model: ${modelId}`);
  settingsStore.currentModelId = modelId;
  // 模型切换逻辑已在 Settings.vue 的 watch 中处理，会自动同步
};

// 自定义样式
const customStyleElement = ref(null);
const stylePath = ref("");

// 加载外部 CSS 样式
async function loadCustomStyle() {
  try {
    stylePath.value = await invoke("get_style_path");
    const cssContent = await readTextFile(stylePath.value);
    applyCustomStyle(cssContent);
  } catch (e) {
    console.error("Failed to load custom style:", e);
  }
}

// 应用自定义样式
function applyCustomStyle(cssContent) {
  // 移除旧的样式元素
  if (customStyleElement.value) {
    customStyleElement.value.remove();
  }
  // 创建新的样式元素
  const style = document.createElement("style");
  style.id = "custom-subtitle-style";
  style.textContent = cssContent;
  document.head.appendChild(style);
  customStyleElement.value = style;
}

// 监听样式文件变化（使用轮询方式）
let styleWatchInterval = null;
let lastStyleContent = "";

async function watchStyleFile() {
  if (!stylePath.value) return;

  try {
    // 记录初始内容
    lastStyleContent = await readTextFile(stylePath.value);

    // 每2秒检查一次文件变化
    styleWatchInterval = setInterval(async () => {
      try {
        const currentContent = await readTextFile(stylePath.value);
        if (currentContent !== lastStyleContent) {
          console.log("Style file changed, reloading...");
          lastStyleContent = currentContent;
          applyCustomStyle(currentContent);
        }
      } catch (e) {
        // 文件可能正在被写入，忽略错误
      }
    }, 2000);
  } catch (e) {
    console.error("Failed to setup style file watch:", e);
  }
}

// 打开样式编辑器
async function openStyleEditor() {
  try {
    await invoke("open_style_editor");
  } catch (e) {
    console.error("Failed to open style editor:", e);
  }
}

// 同步模型配置到后端
async function syncModelConfigToBackend() {
  const currentModel = settingsStore.currentModel;
  if (!currentModel) {
    console.log("No model configured, skipping sync");
    return;
  }

  try {
    const config = {
      current_model_id: currentModel.id,
      models: [{
        id: currentModel.id,
        name: currentModel.model_name,
        model_dir: currentModel.model_dir,
        model_type: {
          type: "Transducer",
          encoder: currentModel.encoder || "",
          decoder: currentModel.decoder || "",
          joiner: currentModel.joiner || "",
        },
        tokens: currentModel.tokens || "",
        languages: ["zh", "en"],
        sample_rate: 16000,
        num_threads: 2,
      }],
    };
    await invoke("update_config", { config });
    console.log("Model config synced to backend:", currentModel.model_name);
  } catch (e) {
    console.error("Failed to sync model config:", e);
  }
}

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

// 隐藏窗口（不退出应用）
async function hideWindow() {
  await appWindow.hide();
}

// 最小化窗口
async function minimizeWindow() {
  await appWindow.minimize();
}

// 开始拖动
function startDrag() {
  if (!isLocked.value) {
    appWindow.startDragging();
  }
}

// 锁定/解锁窗口
async function toggleLock() {
  isLocked.value = !isLocked.value;
  await appWindow.setResizable(!isLocked.value);
}

// 切换历史字幕显示
function toggleHistory() {
  settingsStore.showHistory = !settingsStore.showHistory;
}

// 监听字幕事件
let unlistenSubtitle = null;
let unlistenError = null;
let unlistenClose = null;

onMounted(async () => {
  // 加载自定义样式
  await loadCustomStyle();
  // 监听样式文件变化
  await watchStyleFile();

  // 如果有持久化的模型配置，同步到 Rust 后端
  await syncModelConfigToBackend();

  // 如果禁用了窗口状态记忆，重置窗口到默认位置
  if (!settingsStore.rememberWindowState) {
    try {
      await appWindow.setSize({ type: 'Physical', width: 800, height: 200 });
      await appWindow.center();
    } catch (e) {
      console.error("Failed to reset window position:", e);
    }
  }

  // 检查初始状态
  try {
    isRunning.value = await invoke("is_recognition_running");
  } catch (e) {
    console.error("Failed to get initial state:", e);
  }

  // 拦截窗口关闭事件，改为隐藏窗口
  unlistenClose = await appWindow.onCloseRequested(async (event) => {
    event.preventDefault();
    await appWindow.hide();
  });

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

  // 自动开始识别（仅在有配置的情况下）
  if (!isRunning.value && settingsStore.currentModel) {
    try {
      await invoke("start_recognition");
      isRunning.value = true;
    } catch (e) {
      // 如果启动失败，可能是配置问题
      errorMessage.value = String(e);
      console.error("Auto start failed:", e);
      // 不是致命错误，用户可以手动启动
    }
  } else if (!settingsStore.currentModel) {
    console.log("No model configured, skipping auto start");
  }
});

onUnmounted(() => {
  if (unlistenSubtitle) unlistenSubtitle();
  if (unlistenError) unlistenError();
  if (unlistenClose) unlistenClose();
  // 清理样式文件监听
  if (styleWatchInterval) clearInterval(styleWatchInterval);
  // 移除自定义样式元素
  if (customStyleElement.value) {
    customStyleElement.value.remove();
  }
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
  if (!settingsStore.showHistory) return [];

  let history;
  if (currentText.value) {
    // 有正在识别的文本，显示所有已完成的字幕
    history = subtitles.value;
  } else {
    // 没有正在识别的文本，最后一条作为最新字幕显示
    if (subtitles.value.length <= 1) return [];
    history = subtitles.value.slice(0, -1);
  }
  return history;
});

// 历史字幕文本（带长度限制）
const historyText = computed(() => {
  const text = historySubtitles.value.map(s => s.text).join(' ');
  if (settingsStore.maxHistoryLength > 0 && text.length > settingsStore.maxHistoryLength) {
    return '...' + text.slice(-settingsStore.maxHistoryLength);
  }
  return text;
});
</script>

<template>
  <div class="app-container">
    <!-- 顶部控制栏（自动隐藏） -->
    <div class="top-bar" :class="{ locked: isLocked }" @mousedown="startDrag">
      <!-- 锁定时只显示解锁按钮 -->
      <template v-if="isLocked">
        <div class="top-bar-center" @mousedown.stop>
          <button class="action-btn unlock-btn" @click="toggleLock" title="解锁窗口">
            <UnlockOutlined />
          </button>
        </div>
      </template>

      <!-- 未锁定时显示所有按钮 -->
      <template v-else>
        <div class="top-bar-left" @mousedown.stop>
          <button class="action-btn" :class="{ active: isRunning }" @click="toggleRecognition"
            :title="isRunning ? '停止识别' : '开始识别'">
            <PauseOutlined v-if="isRunning" />
            <CaretRightOutlined v-else />
          </button>
          <button class="action-btn" :class="{ active: settingsStore.showHistory }" @click="toggleHistory"
            :title="settingsStore.showHistory ? '隐藏历史' : '显示历史'">
            <MessageOutlined v-if="settingsStore.showHistory" />
            <CommentOutlined v-else />
          </button>
          <button class="action-btn" @click="copyAllText" title="复制全部">
            <CopyOutlined />
          </button>
          <button class="action-btn" @click="clearSubtitles" title="清空字幕">
            <DeleteOutlined />
          </button>
          <button class="action-btn" @click="toggleLock" title="锁定窗口">
            <LockOutlined />
          </button>
          <button class="action-btn" @click="openStyleEditor" title="编辑样式">
            <FormatPainterOutlined />
          </button>
        </div>
        <div class="top-bar-right" @mousedown.stop>
          <button class="control-btn" @click="minimizeWindow" title="最小化">
            <MinusOutlined />
          </button>
          <button class="control-btn close-btn" @click="hideWindow" title="隐藏到托盘">
            <CloseOutlined />
          </button>
        </div>
      </template>
    </div>

    <!-- 字幕区域 -->
    <div class="subtitle-area" @mouseenter="isHovering = true" @mouseleave="isHovering = false">
      <!-- 历史字幕（合并显示，可滚动） -->
      <div class="history-text" v-if="historyText">
        {{ historyText }}
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

      <!-- 模型下拉列表（右下角，悬停时显示） -->
      <div class="model-selector" v-show="isHovering || isSelectOpen" @mousedown.stop @mouseenter="isHovering = true"
        @mouseleave="isHovering = false">
        <Select v-model:value="settingsStore.currentModelId" :options="modelOptions" placeholder="选择模型" size="small"
          @change="switchModel" @dropdownVisibleChange="isSelectOpen = $event"
          :getPopupContainer="(trigger) => trigger.parentElement" />
      </div>
    </div>
  </div>
</template>

<style></style>

<!-- 字幕样式从外部 CSS 文件动态加载，支持用户自定义和热更新 -->
