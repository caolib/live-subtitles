import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useWindowControl(settingsStore) {
    // 状态
    const isLocked = ref(false); // 窗口锁定状态
    const isHovering = ref(false); // 鼠标是否在窗口上
    const isExpanded = ref(false); // 窗口是否展开（显示工具栏）
    const isMaximized = ref(false); // 是否最大化

    // 内部变量
    const appWindow = getCurrentWindow();
    let lastDragTime = 0;
    let unlistenResize = null;

    // 处理鼠标进入
    async function handleMouseEnter() {
        isHovering.value = true;
        isExpanded.value = true;
    }

    // 处理鼠标离开
    async function handleMouseLeave() {
        // 如果是刚刚触发了拖动，忽略此次离开事件（Tauri startDragging 会导致 mouseleave）
        if (Date.now() - lastDragTime < 200) {
            return;
        }

        isHovering.value = false;

        // 延迟隐藏
        setTimeout(() => {
            if (!isHovering.value) {
                isExpanded.value = false;
            }
        }, 100);
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
    async function toggleMaximize() {
        await appWindow.toggleMaximize();
        isMaximized.value = await appWindow.isMaximized();
    }

    // 开始拖动
    function startDrag(event) {
        if (!isLocked.value) {
            // 如果点击的是文本或可选择元素，不触发拖动
            if (event && event.target) {
                const style = window.getComputedStyle(event.target);
                // 检查是否有 user-select: text (默认 auto 也是 text)
                // 简单判断：如果 cursor 是 text，或者在 .current-subtitle / .history-text内
                if (style.cursor === 'text' ||
                    event.target.closest('.current-subtitle') ||
                    event.target.closest('.history-text') ||
                    style.userSelect === 'text') {
                    return;
                }
            }

            lastDragTime = Date.now();
            appWindow.startDragging();
        }
    }

    // 锁定/解锁窗口
    async function toggleLock() {
        isLocked.value = !isLocked.value;
        await appWindow.setResizable(!isLocked.value);
    }

    /**
     * 开始调整窗口大小
     * @param {string} direction - 调整方向: 'Top', 'Bottom', 'Left', 'Right', 'TopLeft', etc.
     */
    async function startResize(direction) {
        if (!isLocked.value) {
            try {
                // Tauri v2 API
                await appWindow.startResizeDragging(direction);
            } catch (e) {
                console.error("Failed to start resize drag:", e);
            }
        }
    }

    onMounted(async () => {
        // 如果禁用了窗口状态记忆，重置窗口到默认位置
        if (settingsStore && !settingsStore.rememberWindowState) {
            try {
                await appWindow.setSize({ type: 'Physical', width: 800, height: 200 });
                await appWindow.center();
            } catch (e) {
                console.error("Failed to reset window position:", e);
            }
        }
    });

    onUnmounted(() => {
        if (unlistenResize) unlistenResize();
    });

    return {
        isLocked,
        isHovering,
        isExpanded,
        isMaximized,
        appWindow,
        handleMouseEnter,
        handleMouseLeave,
        hideWindow,
        minimizeWindow,
        toggleMaximize,
        startDrag,
        toggleLock,
        startResize,
    };
}
