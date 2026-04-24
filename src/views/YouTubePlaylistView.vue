<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { usePlayerStore, type TrackInfo } from '@/stores/player'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()
const router = useRouter()
const player = usePlayerStore()
const { t } = useI18n()

const isLoading = ref(true)
const error = ref<string | null>(null)
const playlistName = ref('')
const subtitle = ref('')
const coverUrl = ref('')
const searchQuery = ref('')

const tracks = ref<TrackInfo[]>([])

const filteredTracks = computed(() => {
  if (!searchQuery.value) return tracks.value
  const q = searchQuery.value.toLowerCase()
  return tracks.value.filter(t =>
    t.title.toLowerCase().includes(q) || t.artist.toLowerCase().includes(q)
  )
})

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000)
  return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, '0')}`
}

// 解析 InnerTube browse 响应中的歌曲列表
function parsePlaylistTracks(data: any): TrackInfo[] {
  const result: TrackInfo[] = []
  try {
    const tabs = data?.contents?.singleColumnBrowseResultsRenderer?.tabs ||
                 data?.contents?.twoColumnBrowseResultsRenderer?.tabs || []

    for (const tab of tabs) {
      const contents = tab?.tabRenderer?.content?.sectionListRenderer?.contents || []
      for (const section of contents) {
        const items = section?.musicShelfRenderer?.contents ||
                      section?.musicPlaylistShelfRenderer?.contents || []
        for (const item of items) {
          const renderer = item?.musicResponsiveListItemRenderer
          if (!renderer) continue

          // 提取 videoId
          const overlay = renderer?.overlay?.musicItemThumbnailOverlayRenderer
          const videoId = overlay?.content?.musicPlayButtonRenderer?.playNavigationEndpoint?.watchEndpoint?.videoId
          if (!videoId) continue

          // 提取标题
          const titleRuns = renderer?.flexColumns?.[0]?.musicResponsiveListItemFlexColumnRenderer?.text?.runs || []
          const title = titleRuns.map((r: any) => r.text).join('')

          // 提取艺术家
          const artistRuns = renderer?.flexColumns?.[1]?.musicResponsiveListItemFlexColumnRenderer?.text?.runs || []
          const artist = artistRuns.map((r: any) => r.text).join('')

          // 提取封面
          const thumbnails = renderer?.thumbnail?.musicThumbnailRenderer?.thumbnail?.thumbnails || []
          const cover = thumbnails[thumbnails.length - 1]?.url || ''

          // 提取时长
          const durationText = renderer?.fixedColumns?.[0]?.musicResponsiveListItemFixedColumnRenderer?.text?.runs?.[0]?.text || ''
          const durationMs = parseDuration(durationText)

          result.push({
            id: `youtube:${videoId}`,
            title,
            artist,
            album: '',
            durationMs,
            coverUrl: cover,
            audioUrl: '',
          })
        }
      }
    }
  } catch {
    // 解析失败返回空
  }

  // 也尝试解析 header 信息
  try {
    const header = data?.header?.musicImmersiveHeaderRenderer ||
                   data?.header?.musicDetailHeaderRenderer ||
                   data?.header?.musicEditablePlaylistDetailHeaderRenderer?.header?.musicDetailHeaderRenderer
    if (header) {
      playlistName.value = header?.title?.runs?.[0]?.text || playlistName.value
      subtitle.value = header?.subtitle?.runs?.map((r: any) => r.text).join('') || ''
      const thumbs = header?.thumbnail?.musicThumbnailRenderer?.thumbnail?.thumbnails ||
                     header?.thumbnail?.croppedSquareThumbnailRenderer?.thumbnail?.thumbnails || []
      if (thumbs.length > 0) coverUrl.value = thumbs[thumbs.length - 1]?.url || coverUrl.value
    }
  } catch {
    // 忽略
  }

  return result
}

function parseDuration(text: string): number {
  if (!text) return 0
  const parts = text.split(':').map(Number)
  if (parts.length === 2) return (parts[0] * 60 + parts[1]) * 1000
  if (parts.length === 3) return (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000
  return 0
}

async function loadDetail() {
  const browseId = route.params.browseId as string
  if (!browseId) return

  isLoading.value = true
  error.value = null

  try {
    const data = await invoke<any>('get_youtube_playlist_detail', { browseId })
    tracks.value = parsePlaylistTracks(data)
  } catch (e: any) {
    error.value = e?.toString() || t('player.load_failed')
  } finally {
    isLoading.value = false
  }
}

function playAll() {
  if (tracks.value.length === 0) return
  player.playAll(tracks.value)
}

function playTrack(track: TrackInfo) {
  player.playAll(filteredTracks.value)
  player.play(track)
}

onMounted(loadDetail)
</script>

<template>
  <div class="detail-view">
    <header class="detail-header">
      <button class="back-btn" @click="router.back()">
        <span class="material-symbols-rounded">arrow_back</span>
      </button>
      <div class="header-search" v-if="!isLoading && tracks.length > 0">
        <span class="material-symbols-rounded search-icon">search</span>
        <input v-model="searchQuery" :placeholder="t('player.search_tracks')" class="search-input" />
      </div>
    </header>

    <div v-if="isLoading" class="state-center">
      <span class="material-symbols-rounded spinning">progress_activity</span>
      <p>{{ t('player.loading') }}</p>
    </div>

    <div v-else-if="error" class="state-center">
      <span class="material-symbols-rounded" style="font-size: 48px; opacity: 0.3">error</span>
      <p>{{ error }}</p>
      <button class="retry-btn" @click="loadDetail">{{ t('player.retry') }}</button>
    </div>

    <template v-else>
      <div class="detail-hero">
        <div class="hero-cover">
          <img v-if="coverUrl" :src="coverUrl" referrerpolicy="no-referrer" />
          <span v-else class="material-symbols-rounded filled" style="font-size: 48px; opacity: 0.3">queue_music</span>
        </div>
        <div class="hero-info">
          <h1 class="hero-title">{{ playlistName }}</h1>
          <p v-if="subtitle" class="hero-creator">{{ subtitle }}</p>
          <p class="hero-meta">{{ t('player.track_count', { count: tracks.length }) }}</p>
          <button class="play-all-btn" @click="playAll">
            <span class="material-symbols-rounded filled">play_arrow</span>
            {{ t('player.play_all') }}
          </button>
        </div>
      </div>

      <div v-if="filteredTracks.length === 0" class="state-center">
        <p>{{ t('player.empty_playlist') }}</p>
      </div>
      <div v-else class="track-list">
        <div
          v-for="(track, index) in filteredTracks"
          :key="track.id"
          class="track-item"
          :class="{ active: player.currentTrack?.id === track.id }"
          @click="playTrack(track)"
        >
          <div class="track-index">
            <div v-if="player.currentTrack?.id === track.id && player.isPlaying" class="equalizer-bars"><span class="bar"/><span class="bar"/><span class="bar"/></div>
            <span v-else class="index-num">{{ index + 1 }}</span>
          </div>
          <div class="track-cover">
            <img v-if="track.coverUrl" :src="track.coverUrl" referrerpolicy="no-referrer" loading="lazy" @error="($event.target as HTMLImageElement).style.display = 'none'" />
            <span v-else class="material-symbols-rounded filled">music_note</span>
          </div>
          <div class="track-info">
            <div class="track-title">{{ track.title }}</div>
            <div class="track-meta">{{ track.artist }}</div>
          </div>
          <div class="track-duration">{{ formatDuration(track.durationMs) }}</div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped lang="scss">
@import '@/styles/detail-view.scss';
</style>
