<script setup lang="ts">
import { ref, computed } from 'vue'
import { usePlayerStore } from '@/stores/player'
import { useI18n } from 'vue-i18n'

const emit = defineEmits<{ expand: [] }>()
const player = usePlayerStore()
const { t } = useI18n()

// 进度条拖拽
const isDraggingProgress = ref(false)
const dragRatio = ref(0)
const progressTrackRef = ref<HTMLDivElement>()

// 拖拽时显示本地值，否则显示 player 真实值
const displayProgress = computed(() =>
  isDraggingProgress.value ? dragRatio.value : player.progress
)

function onProgressPointerDown(e: PointerEvent) {
  e.stopPropagation()
  isDraggingProgress.value = true
  const el = progressTrackRef.value!
  el.setPointerCapture(e.pointerId)
  updateDragRatio(e)
}

function onProgressPointerMove(e: PointerEvent) {
  if (!isDraggingProgress.value) return
  updateDragRatio(e)
}

function onProgressPointerUp(e: PointerEvent) {
  if (!isDraggingProgress.value) return
  updateDragRatio(e)
  // 释放后才真正 seek
  player.seekTo(dragRatio.value * player.durationMs)
  isDraggingProgress.value = false
  progressTrackRef.value?.releasePointerCapture(e.pointerId)
}

function updateDragRatio(e: PointerEvent) {
  const el = progressTrackRef.value!
  const rect = el.getBoundingClientRect()
  dragRatio.value = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
}
</script>

<template>
  <div class="mini-player" @click="emit('expand')">
    <div
      ref="progressTrackRef"
      class="progress-track"
      :class="{ dragging: isDraggingProgress }"
      @pointerdown="onProgressPointerDown"
      @pointermove="onProgressPointerMove"
      @pointerup="onProgressPointerUp"
      @pointercancel="onProgressPointerUp"
      @click.stop
    >
      <div class="progress-fill" :style="{ width: `${displayProgress * 100}%` }" />
    </div>

    <div class="mini-body">
      <div class="mini-cover" :class="{ playing: player.isPlaying }">
        <img
          v-if="player.currentTrack?.coverUrl"
          :src="player.currentTrack.coverUrl"
          referrerpolicy="no-referrer"
          class="mini-cover-img"
        />
        <span v-else class="material-symbols-rounded filled" style="font-size: 20px; opacity: 0.6">music_note</span>
      </div>

      <div class="mini-info">
        <div class="mini-title">{{ player.currentTrack?.title || t('player.not_playing') }}</div>
        <div class="mini-artist">{{ player.currentTrack?.artist || '' }}</div>
      </div>

      <div class="mini-actions" @click.stop>
        <button class="mini-btn" @click="player.togglePlayPause()" :disabled="player.isLoadingAudio">
          <transition name="mini-icon" mode="out-in">
            <span
              v-if="player.isLoadingAudio"
              class="material-symbols-rounded spinning mini-play-icon"
              key="loading"
            >progress_activity</span>
            <span
              v-else
              class="material-symbols-rounded filled mini-play-icon"
              :key="player.isPlaying ? 'p' : 'r'"
            >{{ player.isPlaying ? 'pause' : 'play_arrow' }}</span>
          </transition>
        </button>
        <button class="mini-btn" @click="player.next()">
          <span class="material-symbols-rounded filled" style="font-size: 22px">skip_next</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.mini-player {
  position: fixed;
  bottom: 0;
  left: 80px;
  right: 0;
  height: 72px;
  background: var(--md-surface-container);
  cursor: pointer;
  z-index: 100;
  border-top: 1px solid var(--md-surface-container-highest);
  transition: background 150ms;

  &:hover { background: var(--md-surface-container-high); }
}

.progress-track {
  height: 3px;
  background: var(--md-surface-container-highest);
  position: relative;
  cursor: pointer;
  touch-action: none;
  // 扩大点击区域（不改变视觉高度）
  &::before {
    content: '';
    position: absolute;
    top: -6px;
    left: 0;
    right: 0;
    bottom: -4px;
  }
  // 拖拽时加粗
  transition: height 120ms ease;
  &.dragging, &:hover {
    height: 5px;
  }
}

.progress-fill {
  height: 100%;
  background: var(--md-primary);
  border-radius: 0 2px 2px 0;
  transition: width 200ms linear;
  position: relative;

  &::after {
    content: '';
    position: absolute;
    right: -1px;
    top: -2px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--md-primary);
    opacity: 0;
    transition: opacity 150ms;
  }

  .mini-player:hover &::after { opacity: 1; }
}

.mini-body {
  display: flex;
  align-items: center;
  padding: 0 20px;
  height: 69px;
  gap: 14px;
}

.mini-cover {
  width: 46px;
  height: 46px;
  border-radius: var(--radius-sm);
  background: var(--md-surface-variant);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: border-radius 500ms var(--ease-standard);

  &.playing { border-radius: var(--radius-md); }
}

.mini-cover-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: inherit;
}

.mini-info {
  flex: 1;
  min-width: 0;
}

.mini-title {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mini-artist {
  font-size: 12px;
  color: var(--md-on-surface-variant);
  line-height: 1.3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mini-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.mini-btn {
  width: 44px;
  height: 44px;
  border-radius: var(--radius-full);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 150ms;
  overflow: hidden;

  &:hover { background: var(--md-surface-variant); }
  &:active { transform: scale(0.9); }
}

.mini-play-icon {
  font-size: 28px;
  display: block;
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

/* 播放/暂停切换动画 */
.mini-icon-enter-active {
  transition: transform 180ms var(--ease-decelerate), opacity 120ms var(--ease-decelerate);
}
.mini-icon-leave-active {
  transition: transform 100ms var(--ease-accelerate), opacity 80ms var(--ease-accelerate);
}
.mini-icon-enter-from { transform: scale(0.6); opacity: 0; }
.mini-icon-leave-to { transform: scale(0.6); opacity: 0; }
</style>
