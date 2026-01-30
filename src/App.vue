<script setup>
import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useSettingsStore } from "./stores/settings";

import { useWindowControl } from "./composables/useWindowControl";
import { useSubtitle } from "./composables/useSubtitle";
import { useStyleManager } from "./composables/useStyleManager";
import { useAppConfig } from "./composables/useAppConfig";

import TopBar from "./components/TopBar.vue";
import SubtitleDisplay from "./components/SubtitleDisplay.vue";

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
  toggleLock
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
  <div class="app-container" :class="{ expanded: isExpanded }" @mouseenter="handleMouseEnter"
    @mouseleave="handleMouseLeave">

    <TopBar 
      :isLocked="isLocked"
      :isRunning="isRunning"
      :showHistory="settingsStore.showHistory"
      :lowercaseSubtitle="settingsStore.lowercaseSubtitle"
      :isMaximized="isMaximized"
      :styleObj="dynamicTopBarStyle"
      @toggleLock="toggleLock"
      @toggleRecognition="toggleRecognition"
      @toggleHistory="toggleHistory"
      @toggleCase="toggleCase"
      @copyAllText="copyAllText"
      @clearSubtitles="clearSubtitles"
      @openStyleEditor="openStyleEditor"
      @openSettings="openSettings"
      @minimizeWindow="minimizeWindow"
      @toggleMaximize="toggleMaximize"
      @hideWindow="hideWindow"
      @startDrag="startDrag"
    />

    <SubtitleDisplay
      :styleObj="dynamicSubtitleAreaStyle"
      :historyText="historyText"
      :historySubtitleStyle="dynamicHistorySubtitleStyle"
      :latestSubtitle="latestSubtitle"
      :currentSubtitleStyle="dynamicCurrentSubtitleStyle"
      :isRunning="isRunning"
      :isModelLoading="isModelLoading"
      :errorMessage="errorMessage"
      :isHovering="isHovering"
      :currentModelName="currentModelName"
    />
  </div>
</template>
