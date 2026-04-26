<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'

const props = defineProps<{
  forceLight?: boolean
  nowPlaying?: boolean
  trackName?: string
  albumName?: string
}>()

const emit = defineEmits<{
  collapse: []
  toggleMore: []
}>()

const { t } = useI18n()

const isMaximized = ref(false)
let unlistenResize: UnlistenFn | null = null

const appWindow = getCurrentWindow()

async function refreshMaximized() {
  try {
    isMaximized.value = await appWindow.isMaximized()
  } catch {
    isMaximized.value = false
  }
}

function minimize() {
  appWindow.minimize().catch(() => {})
}

function toggleMaximize() {
  appWindow.toggleMaximize().then(refreshMaximized).catch(() => {})
}

function close() {
  appWindow.close().catch(() => {})
}

onMounted(async () => {
  await refreshMaximized()
  try {
    unlistenResize = await appWindow.onResized(() => refreshMaximized())
  } catch {}
})

onUnmounted(() => {
  if (unlistenResize) unlistenResize()
})
</script>

<template>
  <header
    class="title-bar"
    :class="{ 'tb-force-light': forceLight, 'tb-np-mode': nowPlaying }"
    data-tauri-drag-region
  >
    <!-- 普通模式：logo + 名称 -->
    <div v-if="!nowPlaying" class="tb-brand" data-tauri-drag-region>
      <img src="/app-icon.png" alt="logo" class="tb-icon" />
      <span class="tb-title">NeriPlayer</span>
    </div>

    <!-- 播放器模式：折叠按钮（覆盖 logo 区域） -->
    <div v-else class="tb-np-left">
      <button class="tb-np-btn tb-np-collapse" type="button" @click="emit('collapse')">
        <span class="material-symbols-rounded">keyboard_arrow_down</span>
      </button>
    </div>

    <!-- 播放器模式：居中播放信息 -->
    <div v-if="nowPlaying" class="tb-np-center" data-tauri-drag-region>
      <span class="tb-np-label">{{ t('player.now_playing') }}</span>
      <span class="tb-np-track">{{ albumName || trackName || '' }}</span>
    </div>

    <!-- 拖拽占位 -->
    <div class="tb-drag" data-tauri-drag-region></div>

    <!-- 播放器模式：更多按钮 -->
    <button v-if="nowPlaying" class="tb-np-btn tb-np-more" type="button" @click="emit('toggleMore')">
      <span class="material-symbols-rounded">more_vert</span>
    </button>

    <!-- 窗口控制 -->
    <div class="tb-controls">
      <button class="tb-ctrl" type="button" @click="minimize" title="最小化">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <rect x="2" y="5.5" width="8" height="1" fill="currentColor" />
        </svg>
      </button>
      <button class="tb-ctrl" type="button" @click="toggleMaximize" :title="isMaximized ? '还原' : '最大化'">
        <svg v-if="!isMaximized" width="12" height="12" viewBox="0 0 12 12">
          <rect x="2.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1" />
        </svg>
        <svg v-else width="12" height="12" viewBox="0 0 12 12">
          <rect x="3.5" y="2" width="6.5" height="6.5" fill="none" stroke="currentColor" stroke-width="1" />
          <rect x="2" y="3.5" width="6.5" height="6.5" fill="var(--md-background)" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
      <button class="tb-ctrl tb-close" type="button" @click="close" title="关闭">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="2.5" y1="2.5" x2="9.5" y2="9.5" stroke="currentColor" stroke-width="1" />
          <line x1="9.5" y1="2.5" x2="2.5" y2="9.5" stroke="currentColor" stroke-width="1" />
        </svg>
      </button>
    </div>
  </header>
</template>

<style scoped lang="scss">
.title-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 36px;
  display: flex;
  align-items: stretch;
  background: transparent;
  color: var(--md-on-surface);
  user-select: none;
  z-index: 1000;
  pointer-events: none;
  transition: color 300ms var(--ease-standard),
              height 300ms var(--ease-standard);

  &.tb-np-mode {
    height: 56px;
    padding-top: 8px;
  }
}

.title-bar.tb-force-light {
  color: rgba(255, 255, 255, 0.9);
}

.tb-brand,
.tb-controls,
.tb-ctrl,
.tb-np-left,
.tb-np-btn,
.tb-np-more {
  pointer-events: auto;
}

/* ========== 普通模式 ========== */
.tb-brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 14px;
  -webkit-app-region: drag;
}

.tb-icon {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  object-fit: contain;
  pointer-events: none;
  opacity: 0.95;
}

.tb-title {
  font-size: 12px;
  font-weight: 600;
  color: inherit;
  letter-spacing: 0.2px;
  pointer-events: none;
  opacity: 0.85;
}

/* ========== 播放器模式 ========== */
.tb-np-left {
  display: flex;
  align-items: center;
  padding: 0 8px;
  -webkit-app-region: no-drag;
}

.tb-np-btn {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: var(--radius-full);
  color: inherit;
  opacity: 0.8;
  cursor: pointer;
  transition: background var(--duration-short) var(--ease-standard),
              opacity var(--duration-short) var(--ease-standard);

  &:hover {
    background: rgba(255, 255, 255, 0.1);
    opacity: 1;
  }

  .material-symbols-rounded {
    font-size: 24px;
  }
}

.tb-np-collapse .material-symbols-rounded {
  font-size: 30px;
}

.tb-np-center {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1px;
  pointer-events: auto;
  -webkit-app-region: drag;
  min-width: 0;
}

.tb-np-label {
  font-size: 10px;
  font-weight: 600;
  color: inherit;
  opacity: 0.45;
  text-transform: uppercase;
  letter-spacing: 1px;
  line-height: 1.2;
}

.tb-np-track {
  font-size: 12px;
  font-weight: 600;
  color: inherit;
  opacity: 0.85;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
  line-height: 1.2;
}

.tb-np-more {
  align-self: center;
  margin-right: 4px;
}

/* 播放器模式下窗口控制按钮适配更高的顶栏 */
.tb-np-mode .tb-ctrl {
  width: 40px;
  height: 40px;
  align-self: center;
  border-radius: var(--radius-full);
}

.tb-np-mode .tb-ctrl svg {
  width: 14px;
  height: 14px;
}

.tb-np-mode .tb-controls {
  align-items: center;
  gap: 2px;
  padding-right: 8px;
}

/* ========== 公共 ========== */
.tb-drag {
  flex: 1;
  -webkit-app-region: drag;
  pointer-events: auto;
}

.tb-controls {
  display: flex;
  align-items: stretch;
  -webkit-app-region: no-drag;
}

.tb-ctrl {
  width: 46px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: inherit;
  opacity: 0.7;
  cursor: pointer;
  transition: background var(--duration-short) var(--ease-standard),
              opacity var(--duration-short) var(--ease-standard);

  &:hover {
    background: rgba(255, 255, 255, 0.08);
    opacity: 1;
  }
}

.light-theme .title-bar:not(.tb-force-light) .tb-ctrl:hover {
  background: rgba(0, 0, 0, 0.06);
}

.light-theme .title-bar:not(.tb-force-light) .tb-np-btn:hover {
  background: rgba(0, 0, 0, 0.06);
}

.tb-close:hover {
  background: #e81123 !important;
  color: #fff;
  opacity: 1;
}
</style>
