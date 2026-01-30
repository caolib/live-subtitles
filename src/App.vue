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
  FormatPainterOutlined,
  BorderOutlined,
  FullscreenExitOutlined,
  LoadingOutlined,
  FontSizeOutlined,
  SettingOutlined
} from "@ant-design/icons-vue";
import { storeToRefs } from "pinia";
import { useSettingsStore } from "./stores/settings";

// Pinia Store
const settingsStore = useSettingsStore();

// 监听字体家族变化，更新 CSS 变量
// 这样可以全局应用字体，无需为每个元素单独设置
watch(
  () => settingsStore.subtitleFontFamily,
  (newVal) => {
    if (newVal) {
      document.documentElement.style.setProperty('--main-font-family', newVal);
    } else {
      document.documentElement.style.removeProperty('--main-font-family');
    }
  },
  { immediate: true }
);

// 使用 storeToRefs 确保响应式
const { currentModelId, currentModel, availableModels } = storeToRefs(settingsStore);

// 状态
const isRunning = ref(false);
const isModelLoading = ref(false); // 模型加载中
const isLocked = ref(false); // 窗口锁定状态
const subtitles = ref([]); // 已完成的字幕历史
const currentText = ref(""); // 正在识别的文本（中间结果）
const maxSubtitles = 5; // 最多显示的字幕条数
const errorMessage = ref("");
const isHovering = ref(false); // 鼠标是否在窗口上
const isExpanded = ref(false); // 窗口是否展开（显示工具栏）
const EXPAND_HEIGHT = 40; // 展开高度 (40px 工具栏，无间隔)
const ANIMATION_DURATION = 300; // 动画时长
let expandTimeout = null;
const appWindow = getCurrentWindow();
let isAnimating = false; // 是否正在进行窗口大小动画
let animationAbortController = null; // 用于中断动画
let baseWindowHeight = null; // 窗口的基准高度（未展开时）
let baseWindowWidth = null; // 窗口的基准宽度

/**
 * 初始化基准窗口尺寸
 */
async function initBaseWindowSize() {
  if (baseWindowHeight === null) {
    const size = await appWindow.innerSize();
    baseWindowWidth = size.width;
    // 假设启动时是未展开状态
    baseWindowHeight = size.height;
  }
}

/**
 * 带有缓动效果的窗口大小调整（基于绝对目标高度）
 * @param {boolean} expanded - 目标状态：true=展开，false=收缩
 * @param {number} duration - 动画持续时间
 * @returns {Promise<boolean>} - 动画是否成功完成（未被中断）
 */
async function animateWindowToState(expanded, duration) {
  // 确保基准高度已初始化
  await initBaseWindowSize();

  const targetHeight = expanded ? baseWindowHeight + EXPAND_HEIGHT : baseWindowHeight;

  // 中断之前的动画
  if (animationAbortController) {
    animationAbortController.abort = true;
  }

  const currentController = { abort: false };
  animationAbortController = currentController;

  isAnimating = true;
  try {
    const startSize = await appWindow.innerSize();
    const startHeight = startSize.height;
    const heightChange = targetHeight - startHeight;

    // 如果高度已经是目标高度，无需动画
    if (Math.abs(heightChange) < 2) {
      return true;
    }

    // 分割步数，每步约 16ms
    const steps = Math.max(10, Math.floor(duration / 16));

    for (let i = 1; i <= steps; i++) {
      // 检查是否被中断
      if (currentController.abort) {
        return false;
      }

      // 简单的 ease-out 算法
      const progress = i / steps;
      const easeProgress = 1 - Math.pow(1 - progress, 3);

      const currentHeight = Math.round(startHeight + (heightChange * easeProgress));

      await appWindow.setSize({
        type: 'Physical',
        width: baseWindowWidth,
        height: currentHeight
      });
    }

    // 检查是否被中断
    if (currentController.abort) {
      return false;
    }

    // 确保最终高度正确
    await appWindow.setSize({
      type: 'Physical',
      width: baseWindowWidth,
      height: targetHeight
    });

    return true;
  } catch (e) {
    console.error("Window animation failed:", e);
    return false;
  } finally {
    if (animationAbortController === currentController) {
      isAnimating = false;
      animationAbortController = null;
    }
  }
}

/**
 * 直接设置窗口到目标高度（基于基准高度）
 * @param {boolean} expanded - 是否展开状态
 */
async function setWindowToTargetHeight(expanded) {
  await initBaseWindowSize();

  const targetHeight = expanded ? baseWindowHeight + EXPAND_HEIGHT : baseWindowHeight;

  await appWindow.setSize({
    type: 'Physical',
    width: baseWindowWidth,
    height: targetHeight
  });
}

// 处理鼠标进入：展开窗口
async function handleMouseEnter() {
  isHovering.value = true;
  if (isLocked.value) return;

  // 如果有正在等待执行的收缩任务，取消它
  if (expandTimeout) {
    clearTimeout(expandTimeout);
    expandTimeout = null;
  }

  // 如果已经是展开显示状态，确保高度正确后返回
  if (isExpanded.value) {
    // 如果正在动画中，中断它并直接设置到目标高度
    if (isAnimating) {
      await setWindowToTargetHeight(true);
    }
    return;
  }

  try {
    // 触发 CSS 动画 (内容下移，工具栏出现)
    isExpanded.value = true;

    // 执行窗口高度增加动画（基于绝对目标高度）
    await animateWindowToState(true, 200);
  } catch (e) {
    console.error("Expand window failed:", e);
  }
}

// 处理鼠标离开：收缩窗口
async function handleMouseLeave() {
  // 如果是刚刚触发了拖动，忽略此次离开事件（Tauri startDragging 会导致 mouseleave）
  if (Date.now() - lastDragTime < 200) {
    return;
  }

  isHovering.value = false;
  if (isLocked.value || !isExpanded.value) return;

  // 先触发 CSS 动画 (内容上移，工具栏消失)
  isExpanded.value = false;

  // 延迟缩小窗口，等待 CSS 动画完成
  expandTimeout = setTimeout(async () => {
    expandTimeout = null;

    // 再次检查拖动状态，防止在拖动过程中定时器触发
    if (Date.now() - lastDragTime < ANIMATION_DURATION + 200) {
      return;
    }

    try {
      // 检查鼠标是否真的离开了，以及是否应该保持收缩状态
      if (isHovering.value || isExpanded.value) {
        return;
      }

      // 执行窗口收缩动画（基于绝对目标高度）
      await animateWindowToState(false, 200);
    } catch (e) {
      console.error("Shrink window failed:", e);
    }
  }, ANIMATION_DURATION);
}

// 当前模型名称 - 从 store 的 currentModel 计算得出（响应式）
const currentModelName = computed(() => {
  const model = currentModel.value;
  const name = model?.model_name || '未配置模型';
  console.log('Computing model name:', name, 'from currentModelId:', currentModelId.value);
  return name;
});

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

// 打开设置窗口
async function openSettings() {
  try {
    await invoke("open_settings");
  } catch (e) {
    console.error("Failed to open settings:", e);
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
    // 获取当前音频源对应的设备ID
    const currentDeviceId = settingsStore.audioSourceType === 'systemaudio'
      ? settingsStore.audioDeviceIdForSystem
      : settingsStore.audioDeviceIdForMicrophone;

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
      // 同步音频源配置
      audio_source_type: settingsStore.audioSourceType,
      audio_device_id: currentDeviceId || "",
    };
    await invoke("update_config", { config });
    console.log("Model config synced to backend:", currentModel.model_name);
    console.log("Audio source synced:", settingsStore.audioSourceType, "Device:", currentDeviceId || "Default");
  } catch (e) {
    console.error("Failed to sync model config:", e);
  }
}

// 拖动相关
// const appWindow = getCurrentWindow();

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

// 最大化/还原窗口
const isMaximized = ref(false);

async function toggleMaximize() {
  await appWindow.toggleMaximize();
  isMaximized.value = await appWindow.isMaximized();
}

// 开始拖动
let lastDragTime = 0;
function startDrag() {
  if (!isLocked.value) {
    lastDragTime = Date.now();
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
let unlistenModelLoading = null;
let unlistenModelSwitched = null;
let unlistenSettingsSync = null;
let unlistenResize = null;

onMounted(async () => {
  // 初始化基准窗口尺寸
  await initBaseWindowSize();

  // 监听窗口大小变化，更新基准尺寸（用户手动调整时）
  unlistenResize = await appWindow.onResized(async ({ payload: size }) => {
    // 如果正在动画中，忽略此次变化（是我们的动画导致的）
    if (isAnimating) return;

    // 用户手动调整了窗口大小，更新基准尺寸
    baseWindowWidth = size.width;
    // 如果当前是展开状态，需要减去展开高度得到基准高度
    baseWindowHeight = isExpanded.value ? size.height - EXPAND_HEIGHT : size.height;
  });

  // 加载自定义样式
  await loadCustomStyle();
  // 监听样式文件变化
  await watchStyleFile();

  // 如果有持久化的模型配置，同步到 Rust 后端
  await syncModelConfigToBackend();

  // 监听设置同步事件（来自 Settings 窗口）
  unlistenSettingsSync = await appWindow.listen('settings-sync', (event) => {
    const appearance = event.payload;
    if (appearance) {
      // 更新本地 store
      settingsStore.updateAppearanceSettings(appearance);
      settingsStore.updateDisplaySettings(appearance); // showHistory 等在这里
    }
  });

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
    isModelLoading.value = false;
  });

  // 监听模型加载状态
  unlistenModelLoading = await listen("model_loading", (event) => {
    isModelLoading.value = event.payload.loading;
  });

  // 监听模型切换事件（从 Settings 窗口发送）
  // 注意：实际的状态同步由 main.js 中的 storage 事件监听器处理
  unlistenModelSwitched = await appWindow.listen('model-switched', (event) => {
    console.log('Received model-switched event:', event.payload);
    console.log('Current model from store:', currentModel.value?.model_name);
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
  if (unlistenModelLoading) unlistenModelLoading();
  if (unlistenModelSwitched) unlistenModelSwitched();
  if (unlistenSettingsSync) unlistenSettingsSync();
  if (unlistenResize) unlistenResize();
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
    return processText(currentText.value);
  }
  if (subtitles.value.length === 0) return "";
  return processText(subtitles.value[subtitles.value.length - 1].text);
});

// 处理文本：根据配置决定是否转小写
function processText(text) {
  if (!text) return text;
  // 如果启用了小写配置，将英文字母转换为小写
  if (settingsStore.lowercaseSubtitle) {
    return text.replace(/[A-Z]/g, char => char.toLowerCase());
  }
  return text;
}

// 切换大小写模式
function toggleCase() {
  settingsStore.lowercaseSubtitle = !settingsStore.lowercaseSubtitle;
}

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
  const text = historySubtitles.value.map(s => processText(s.text)).join(' ');
  if (settingsStore.maxHistoryLength > 0 && text.length > settingsStore.maxHistoryLength) {
    return '...' + text.slice(-settingsStore.maxHistoryLength);
  }
  return text;
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
  // 如果已经是 rgba 或其他格式，直接返回
  return hex;
}

// 动态计算的样式 - 字幕区域背景
// 动态计算的样式 - 顶部工具栏
const dynamicTopBarStyle = computed(() => {
  const bg = settingsStore.subtitleBackgroundColor;
  const opacity = settingsStore.subtitleBackgroundOpacity;
  const backgroundColor = hexToRgba(bg, opacity);

  return {
    backgroundColor: backgroundColor,
  };
});

// 动态计算的样式 - 字幕区域
const dynamicSubtitleAreaStyle = computed(() => {
  const bg = settingsStore.subtitleBackgroundColor;
  const opacity = settingsStore.subtitleBackgroundOpacity;
  const backgroundColor = hexToRgba(bg, opacity);

  const style = {
    backgroundColor: backgroundColor,
  };

  return style;
});

// 动态计算的样式 - 当前字幕
const dynamicCurrentSubtitleStyle = computed(() => {
  const style = {
    fontSize: settingsStore.subtitleFontSize + 'px',
    color: settingsStore.subtitleColor,
  };

  return style;
});

// 动态计算的样式 - 历史字幕
const dynamicHistorySubtitleStyle = computed(() => {
  const style = {
    // 历史字幕字体为当前字体的 70%，且只有透明度变化
    fontSize: Math.max(12, settingsStore.subtitleFontSize * 0.7) + 'px',
    color: settingsStore.subtitleColor,
    opacity: 0.6
  };

  return style;
});

</script>

<template>
  <div class="app-container" :class="{ expanded: isExpanded }" @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave">
    <!-- 顶部控制栏（自动隐藏） -->
    <div class="top-bar" :class="{ locked: isLocked }" :style="dynamicTopBarStyle" @mousedown="startDrag">
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
          <button class="action-btn" :class="{ active: settingsStore.lowercaseSubtitle }" @click="toggleCase"
            :title="settingsStore.lowercaseSubtitle ? '当前:小写' : '当前:原始'">
            <FontSizeOutlined />
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
          <button class="action-btn" @click="openSettings" title="设置">
            <SettingOutlined />
          </button>
        </div>
        <div class="top-bar-right" @mousedown.stop>
          <button class="control-btn" @click="minimizeWindow" title="最小化">
            <MinusOutlined />
          </button>
          <button class="control-btn" @click="toggleMaximize" :title="isMaximized ? '还原' : '最大化'">
            <FullscreenExitOutlined v-if="isMaximized" />
            <BorderOutlined v-else />
          </button>
          <button class="control-btn close-btn" @click="hideWindow" title="隐藏到托盘">
            <CloseOutlined />
          </button>
        </div>
      </template>
    </div>

    <!-- 字幕区域 -->
    <div class="subtitle-area" :style="dynamicSubtitleAreaStyle">
      <!-- 历史字幕（合并显示，可滚动） -->
      <div class="history-text" v-if="historyText" :style="dynamicHistorySubtitleStyle">
        {{ historyText }}
      </div>

      <!-- 当前字幕（固定在底部） -->
      <div class="current-subtitle" v-if="latestSubtitle" :style="dynamicCurrentSubtitleStyle">
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
  </div>
</template>
