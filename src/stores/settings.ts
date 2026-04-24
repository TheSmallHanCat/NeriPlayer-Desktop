import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

// tauri-plugin-store 的前端 API
// 首次加载时先用 localStorage fallback，后续接入 tauri store
function loadSetting<T>(key: string, defaultValue: T): T {
  try {
    const stored = localStorage.getItem(`neri:${key}`)
    if (stored !== null) return JSON.parse(stored)
  } catch {}
  return defaultValue
}

function saveSetting(key: string, value: any) {
  localStorage.setItem(`neri:${key}`, JSON.stringify(value))
}

export const useSettingsStore = defineStore('settings', () => {
  // 外观
  const darkMode = ref<'system' | 'dark' | 'light'>(loadSetting('dark_mode', 'dark'))
  const themeColor = ref(loadSetting('theme_color', '#6750A4'))

  // 个性化
  const defaultScreen = ref(loadSetting('default_screen', 'home'))
  const showCoverBadge = ref(loadSetting('cover_badge', true))
  const showNowPlayingTitle = ref(loadSetting('np_title', true))
  const showToolbarDock = ref(loadSetting('np_toolbar', true))
  const showQualitySwitch = ref(loadSetting('quality_switch', true))
  const showAudioCodec = ref(loadSetting('audio_codec', true))
  const showAudioSpec = ref(loadSetting('audio_spec', true))
  const lyricFontScale = ref(loadSetting('lyric_font_scale', 1.0))

  // 播放
  const crossfade = ref(loadSetting('crossfade', false))
  const normalizeVolume = ref(loadSetting('normalize', false))
  const fadeIn = ref(loadSetting('fade_in', false))
  const fadeInDuration = ref(loadSetting('fade_in_duration', 500))
  const fadeOutDuration = ref(loadSetting('fade_out_duration', 500))
  const crossfadeNext = ref(loadSetting('crossfade_next', false))
  const crossfadeInDuration = ref(loadSetting('crossfade_in_duration', 500))
  const crossfadeOutDuration = ref(loadSetting('crossfade_out_duration', 500))
  const keepProgress = ref(loadSetting('keep_progress', true))
  const keepPlaybackMode = ref(loadSetting('keep_mode', true))

  // 歌词
  const showTranslation = ref(loadSetting('show_translation', true))
  const lyricBlur = ref(loadSetting('lyric_blur', true))
  const lyricBlurAmount = ref(loadSetting('lyric_blur_amount', 1.5))
  const cloudMusicOffset = ref(loadSetting('cloud_offset', 0))
  const qqMusicOffset = ref(loadSetting('qq_offset', 0))

  // 动效
  const advancedLyrics = ref(loadSetting('advanced_lyrics', true))
  const dynamicBackground = ref(loadSetting('dynamic_bg', true))
  const audioReactive = ref(loadSetting('audio_reactive', true))
  const coverBlurBg = ref(loadSetting('cover_blur_bg', false))
  const coverBlurAmount = ref(loadSetting('cover_blur_amount', 1.5))
  const coverBlurDarken = ref(loadSetting('cover_blur_darken', 0.2))

  // 音质
  const neteaseQuality = ref(loadSetting('netease_quality', 'exhigh'))
  const youtubeQuality = ref(loadSetting('youtube_quality', 'very_high'))
  const biliQuality = ref(loadSetting('bili_quality', 'high'))

  // 存储
  const maxCacheSize = ref(loadSetting('cache_size', 1024))
  const downloadNameTemplate = ref(loadSetting('download_template', '{artist} - {title}'))

  // 自动持久化：watch 所有 ref，变化时保存
  const allSettings: Record<string, any> = {
    dark_mode: darkMode, theme_color: themeColor,
    default_screen: defaultScreen, cover_badge: showCoverBadge,
    np_title: showNowPlayingTitle, np_toolbar: showToolbarDock,
    quality_switch: showQualitySwitch, audio_codec: showAudioCodec,
    audio_spec: showAudioSpec, lyric_font_scale: lyricFontScale,
    crossfade, normalize: normalizeVolume,
    fade_in: fadeIn, fade_in_duration: fadeInDuration,
    fade_out_duration: fadeOutDuration, crossfade_next: crossfadeNext,
    crossfade_in_duration: crossfadeInDuration, crossfade_out_duration: crossfadeOutDuration,
    keep_progress: keepProgress, keep_mode: keepPlaybackMode,
    show_translation: showTranslation, lyric_blur: lyricBlur,
    lyric_blur_amount: lyricBlurAmount, cloud_offset: cloudMusicOffset,
    qq_offset: qqMusicOffset, advanced_lyrics: advancedLyrics,
    dynamic_bg: dynamicBackground, audio_reactive: audioReactive,
    cover_blur_bg: coverBlurBg, cover_blur_amount: coverBlurAmount,
    cover_blur_darken: coverBlurDarken,
    netease_quality: neteaseQuality, youtube_quality: youtubeQuality,
    bili_quality: biliQuality, cache_size: maxCacheSize,
    download_template: downloadNameTemplate,
  }

  for (const [key, refVal] of Object.entries(allSettings)) {
    watch(refVal, (val) => saveSetting(key, val))
  }

  return {
    darkMode, themeColor,
    defaultScreen, showCoverBadge, showNowPlayingTitle, showToolbarDock,
    showQualitySwitch, showAudioCodec, showAudioSpec, lyricFontScale,
    crossfade, normalizeVolume,
    fadeIn, fadeInDuration, fadeOutDuration,
    crossfadeNext, crossfadeInDuration, crossfadeOutDuration,
    keepProgress, keepPlaybackMode,
    showTranslation, lyricBlur, lyricBlurAmount,
    cloudMusicOffset, qqMusicOffset,
    advancedLyrics, dynamicBackground, audioReactive,
    coverBlurBg, coverBlurAmount, coverBlurDarken,
    neteaseQuality, youtubeQuality, biliQuality,
    maxCacheSize, downloadNameTemplate,
  }
})
