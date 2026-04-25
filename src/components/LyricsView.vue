<script setup lang="ts">
/**
 * LyricsView — Web Animation API 驱动的高性能歌词组件
 * 逐字动画由 KaraokeLine 类管理（移植自 AMLL），Vue 只做行级调度
 */
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from 'vue'
import type { LyricLine } from '@/stores/player'
import { useSettingsStore } from '@/stores/settings'
import { KaraokeLine } from '@/utils/karaokeLine'

const settings = useSettingsStore()

const props = withDefaults(defineProps<{
  lyrics: LyricLine[]
  currentTimeMs: number
  previewTimeMs?: number | null
  isPlaying: boolean
}>(), {
  currentTimeMs: 0,
  previewTimeMs: null,
  isPlaying: false,
})

const emit = defineEmits<{ seek: [timeMs: number] }>()
const containerRef = ref<HTMLDivElement>()

// --- KaraokeLine 实例管理 ---
const karaokeLines = ref<Map<number, KaraokeLine>>(new Map())
let lastActiveIndex = -1

function hasWordTiming(line: LyricLine): boolean {
  return line.words && line.words.length > 0 && line.words.some(w => w.durationMs > 0)
}

function buildKaraokeLines() {
  // 清理旧实例
  for (const kl of karaokeLines.value.values()) kl.dispose()
  karaokeLines.value.clear()

  if (!containerRef.value) return

  // 为有逐字数据的行创建 KaraokeLine
  const lineEls = containerRef.value.querySelectorAll('.lyric-line')
  props.lyrics.forEach((line, i) => {
    if (!hasWordTiming(line)) return
    const lineEl = lineEls[i] as HTMLElement
    if (!lineEl) return

    const wordContainer = lineEl.querySelector('.kw-container') as HTMLElement
    if (!wordContainer) return

    const kl = new KaraokeLine()
    const lineEnd = line.startMs + line.durationMs
    kl.build(wordContainer, line.words, line.startMs, lineEnd)
    karaokeLines.value.set(i, kl)
  })
}

// --- 手动滚动检测 ---
let isAutoScrolling = false
const isUserScrolling = ref(false)
const clearTextHoldIndex = ref<number | null>(null)
let scrollEndTimer: ReturnType<typeof setTimeout> | null = null

function onScroll() {
  if (isAutoScrolling) return
  isUserScrolling.value = true
  clearTextHoldIndex.value = activeIndex.value
  if (scrollEndTimer) clearTimeout(scrollEndTimer)
  scrollEndTimer = setTimeout(() => { isUserScrolling.value = false }, 150)
}

const isClearText = computed(() =>
  isUserScrolling.value || clearTextHoldIndex.value === activeIndex.value
)

// --- 时间 ---
const offsetMs = computed(() => settings.cloudMusicOffset || 0)
const effectiveTimeMs = computed(() =>
  props.previewTimeMs != null ? props.previewTimeMs : props.currentTimeMs
)
const adjustedTimeMs = computed(() => effectiveTimeMs.value + offsetMs.value)

const activeIndex = computed(() => {
  if (!props.lyrics.length) return -1
  const t = adjustedTimeMs.value
  for (let i = props.lyrics.length - 1; i >= 0; i--) {
    if (t >= props.lyrics[i].startMs) return i
  }
  return -1
})

// --- 滚动 ---
function scrollToActive(idx: number, behavior: ScrollBehavior = 'smooth') {
  if (idx < 0 || !containerRef.value) return
  isAutoScrolling = true
  nextTick(() => {
    const lineEls = containerRef.value!.querySelectorAll('.lyric-line')
    const el = lineEls[idx] as HTMLElement
    if (!el) { isAutoScrolling = false; return }
    const target = el.offsetTop - containerRef.value!.clientHeight * 0.30
    containerRef.value!.scrollTo({ top: target, behavior })
    setTimeout(() => { isAutoScrolling = false }, behavior === 'instant' ? 50 : 500)
  })
}

// --- 行级 enable/disable 调度 ---
watch(activeIndex, (idx) => {
  if (clearTextHoldIndex.value !== null && idx !== clearTextHoldIndex.value) {
    clearTextHoldIndex.value = null
  }
  if (!isUserScrolling.value) scrollToActive(idx)

  // 停用上一行
  if (lastActiveIndex >= 0 && lastActiveIndex !== idx) {
    karaokeLines.value.get(lastActiveIndex)?.disable()
  }
  // 激活新行
  if (idx >= 0) {
    karaokeLines.value.get(idx)?.enable(adjustedTimeMs.value, props.isPlaying)
  }
  lastActiveIndex = idx
})

// seek 时定位当前行
watch(adjustedTimeMs, (t) => {
  if (activeIndex.value >= 0) {
    karaokeLines.value.get(activeIndex.value)?.seek(t)
  }
})

// 播放/暂停时同步动画状态
watch(() => props.isPlaying, (playing) => {
  if (activeIndex.value >= 0) {
    const kl = karaokeLines.value.get(activeIndex.value)
    if (playing) kl?.resume()
    else kl?.pause()
  }
})

// 歌词数据变化时重建
watch(() => props.lyrics, () => {
  nextTick(() => buildKaraokeLines())
}, { deep: false })

onMounted(() => {
  containerRef.value?.addEventListener('scroll', onScroll, { passive: true })
  glowAnimFrame = requestAnimationFrame(updateGlow)
  nextTick(() => {
    buildKaraokeLines()
    scrollToActive(activeIndex.value, 'instant')
    if (activeIndex.value >= 0) {
      karaokeLines.value.get(activeIndex.value)?.enable(adjustedTimeMs.value, props.isPlaying)
      lastActiveIndex = activeIndex.value
    }
  })
})

onUnmounted(() => {
  containerRef.value?.removeEventListener('scroll', onScroll)
  if (scrollEndTimer) clearTimeout(scrollEndTimer)
  cancelAnimationFrame(glowAnimFrame)
  for (const kl of karaokeLines.value.values()) kl.dispose()
})

function dist(index: number): number {
  if (activeIndex.value < 0) return 0
  return Math.abs(index - activeIndex.value)
}

// 有符号距离：正数=在活跃行下方，负数=在活跃行上方
function signedDist(index: number): number {
  if (activeIndex.value < 0) return 0
  return index - activeIndex.value
}

function blurForDist(d: number): number {
  if (!settings.lyricBlur || d === 0) return 0
  // 对齐 Android blurForDistance: maxBlur * multiplier
  // maxBlur = lyricBlurAmount (settings, 默认 1.5) 作为基准
  const maxBlur = settings.lyricBlurAmount
  if (d === 1) return maxBlur * 1.0
  if (d === 2) return maxBlur * 1.5
  if (d === 3) return maxBlur * 2.0
  if (d === 4) return maxBlur * 2.5
  return maxBlur * 4.0
}

function scaleForDist(d: number): number {
  // 对齐 Android LyricVisualSpec + farScaleFalloffPerStep
  if (d === 0) return 1.06   // activeScale
  if (d === 1) return 0.95   // nearScale
  // farScale - (d-2) * falloff, coerceIn(farScaleMin, farScale)
  return Math.max(0.80, 0.88 - (d - 2) * 0.02)
}

function alphaForDist(d: number, hasBlur: boolean): number {
  if (d === 0) return 0.95
  if (d === 1) return hasBlur ? 0.72 : 0.4
  const base = hasBlur ? 0.40 : 0.35
  return Math.max(0.16, base - 0.08 * (d - 2))
}

// 3D 翻页倾斜角度（对齐 Android 9deg）
function tiltForLine(index: number): string {
  if (activeIndex.value < 0) return 'rotateX(0)'
  const sd = signedDist(index)
  if (sd === 0) return 'rotateX(0)'
  if (sd < 0) return 'rotateX(9deg)'   // 上方行向上翻
  return 'rotateX(-9deg)'               // 下方行向下翻
}

function tiltOriginForLine(index: number): string {
  if (activeIndex.value < 0) return 'left center'
  const sd = signedDist(index)
  if (sd < 0) return 'left bottom'   // 上方行以底边为轴
  if (sd > 0) return 'left top'      // 下方行以顶边为轴
  return 'left center'
}

// --- 径向辉光跟踪（跟随 karaoke 进度） ---
const glowX = ref(0)  // 辉光 X 位置百分比 0..100
let glowAnimFrame = 0
let glowSmoothed = 0

function updateGlow() {
  const idx = activeIndex.value
  if (idx >= 0) {
    const kl = karaokeLines.value.get(idx)
    if (kl && typeof kl.getProgress === 'function') {
      const target = kl.getProgress() * 100
      // 平滑跟随 110ms
      glowSmoothed += (target - glowSmoothed) * 0.15
      glowX.value = glowSmoothed
    }
  }
  glowAnimFrame = requestAnimationFrame(updateGlow)
}

function seekToLine(line: LyricLine) {
  clearTextHoldIndex.value = null
  emit('seek', line.startMs)
}
</script>

<template>
  <div class="lyrics-scroll" ref="containerRef" :style="{ '--lyric-font-scale': settings.lyricFontScale, perspective: '800px' }">
    <div class="lyrics-pad-top" />

    <div
      v-for="(line, i) in lyrics"
      :key="i"
      class="lyric-line"
      :class="{
        active: i === activeIndex,
        past: activeIndex >= 0 && i < activeIndex,
        'clear-text': isClearText,
      }"
      :style="isClearText ? {} : {
        '--blur': `${blurForDist(dist(i))}px`,
        '--scale': String(scaleForDist(dist(i))),
        '--alpha': String(alphaForDist(dist(i), settings.lyricBlur)),
        '--tilt': tiltForLine(i),
        '--tilt-origin': tiltOriginForLine(i),
      }"
      @click="seekToLine(line)"
    >
      <!-- 径向辉光（仅活跃行） -->
      <div
        v-if="i === activeIndex"
        class="lyric-glow"
        :style="{ '--glow-x': glowX + '%' }"
      />

      <!-- 逐字模式：KaraokeLine 直接操作这个容器的 DOM -->
      <span v-if="hasWordTiming(line)" class="line-text kw-container" />

      <!-- 整行模式 -->
      <span v-else class="line-text">{{ line.text }}</span>

      <!-- 翻译 — 所有行都显示 -->
      <span
        v-if="line.translation && settings.showTranslation"
        class="line-tl"
      >{{ line.translation }}</span>
    </div>

    <div class="lyrics-pad-bottom" />
  </div>
</template>

<style scoped lang="scss">
.lyrics-scroll {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 24px;
  mask-image: linear-gradient(to bottom, transparent 0%, black 20px, black calc(100% - 100px), transparent 100%);
  -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 20px, black calc(100% - 100px), transparent 100%);
  perspective: 800px;
  &::-webkit-scrollbar { display: none; }
  scrollbar-width: none;
}

.lyrics-pad-top { height: 30%; }
.lyrics-pad-bottom { height: 50%; }

.lyric-line {
  position: relative;
  padding: 8px 16px;
  transform-origin: var(--tilt-origin, left center);
  transform: scale(var(--scale, 1)) var(--tilt, rotateX(0));
  opacity: var(--alpha, 1);
  filter: blur(var(--blur, 0px));
  transition:
    transform 260ms cubic-bezier(0.2, 0, 0, 1),
    opacity 400ms cubic-bezier(0.4, 0, 0.2, 1),
    filter 300ms ease-out;
  cursor: pointer;
  will-change: transform, opacity, filter;

  &:hover {
    opacity: 0.6 !important;
    filter: blur(0px) !important;
  }
  &.active {
    filter: none;
    transform-origin: left center;
  }
  &.clear-text {
    transform: none;
    opacity: 0.5;
    filter: none;
    transition: opacity 0.15s;
    &.active { opacity: 1; }
  }
}

// 径向辉光（活跃行，跟随 karaoke 进度）
.lyric-glow {
  position: absolute;
  top: 50%;
  left: var(--glow-x, 0%);
  width: 96px;
  height: 96px;
  transform: translate(-50%, -50%);
  background: radial-gradient(circle, var(--np-primary, rgba(255,255,255,0.85)) 0%, transparent 70%);
  opacity: 0.35;
  pointer-events: none;
  filter: blur(24px);
  will-change: left, opacity;
  transition: opacity 0.3s ease;
  animation: glow-pulse 1.2s ease-in-out infinite alternate;
}

@keyframes glow-pulse {
  from { transform: translate(-50%, -50%) scale(1); opacity: 0.3; }
  to { transform: translate(-50%, -50%) scale(1.15); opacity: 0.45; }
}

.line-text {
  display: block;
  font-size: calc(24px * var(--lyric-font-scale, 1));
  font-weight: 700;
  line-height: 1.5;
  letter-spacing: -0.2px;
  color: rgba(255, 255, 255, 0.5);
  white-space: pre-wrap;
  transition: color 0.4s;
  position: relative;
  z-index: 1;
  .active & { color: white; }
  .clear-text & { color: rgba(255, 255, 255, 0.45); }
  .clear-text.active & { color: white; }
}

// KaraokeLine 创建的逐字 <span> 样式
:deep(.kw) {
  display: inline;
  color: inherit;
  // mask 由 Web Animation API 驱动，此处不需要 transition
}

.line-tl {
  display: block;
  font-size: calc(16px * var(--lyric-font-scale, 1));
  font-weight: 400;
  color: rgba(255, 255, 255, 0.25);
  margin-top: 4px;
  line-height: 1.35;
  position: relative;
  z-index: 1;

  .active & { color: rgba(255, 255, 255, 0.85); }
  .past & { color: rgba(255, 255, 255, 0.2); }
}

.tl-fade-enter-active { transition: opacity 0.25s ease; }
.tl-fade-leave-active { transition: opacity 0.15s ease; }
.tl-fade-enter-from,
.tl-fade-leave-to { opacity: 0; }
</style>
