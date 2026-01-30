import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useWindowControl(settingsStore) {
    // 状态
    const isLocked = ref(false); // 窗口锁定状态
    const isHovering = ref(false); // 鼠标是否在窗口上
    const isExpanded = ref(false); // 窗口是否展开（显示工具栏）
    const isMaximized = ref(false); // 是否最大化

    // 常量
    const EXPAND_HEIGHT = 40; // 展开高度 (40px 工具栏，无间隔)
    const ANIMATION_DURATION = 300; // 动画时长

    // 内部变量
    let expandTimeout = null;
    const appWindow = getCurrentWindow();
    let isAnimating = false; // 是否正在进行窗口大小动画
    let animationAbortController = null; // 用于中断动画
    let baseWindowHeight = null; // 窗口的基准高度（未展开时）
    let baseWindowWidth = null; // 窗口的基准宽度
    let lastDragTime = 0;
    let unlistenResize = null;

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
            // 使用当前宽度，防止重置用户调整过的宽度
            const currentWidth = startSize.width;
            const startHeight = startSize.height;

            // 如果高度已经是目标高度，无需动画
            if (Math.abs(targetHeight - startHeight) < 2) {
                return true;
            }

            // 直接设置窗口大小，移除循环动画以消除卡顿
            await appWindow.setSize({
                type: 'Physical',
                width: currentWidth,
                height: targetHeight
            });

            return true;
        } catch (e) {
            console.error("Window resize failed:", e);
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

        const startSize = await appWindow.innerSize();

        await appWindow.setSize({
            type: 'Physical',
            width: startSize.width,
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

    onMounted(async () => {
        // 初始化基准窗口尺寸
        await initBaseWindowSize();

        // 监听窗口大小变化
        unlistenResize = await appWindow.onResized(async ({ payload: size }) => {
            // 如果正在动画中，忽略此次变化
            if (isAnimating) return;

            // 用户手动调整了窗口大小，更新基准尺寸
            baseWindowWidth = size.width;
            // 如果当前是展开状态，需要减去展开高度得到基准高度
            baseWindowHeight = isExpanded.value ? size.height - EXPAND_HEIGHT : size.height;
        });

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
    };
}
