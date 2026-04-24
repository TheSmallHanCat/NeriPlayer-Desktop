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
const folderName = ref('')
const coverUrl = ref('')
const mediaCount = ref(0)
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

async function loadDetail() {
  const mediaId = Number(route.params.mediaId)
  if (!mediaId) return

  isLoading.value = true
  error.value = null

  try {
    // 获取收藏夹信息
    const infoData = await invoke<any>('get_bili_fav_folder_info', { mediaId })
    const info = infoData?.data || {}
    folderName.value = info.title || ''
    coverUrl.value = info.cover || ''
    mediaCount.value = info.media_count || 0

    // 获取收藏夹内容（分页加载所有）
    let page = 1
    const allItems: any[] = []
    let hasMore = true

    while (hasMore) {
      const data = await invoke<any>('get_bili_favorite_items', { mediaId, page })
      const items = data?.data?.medias || []
      allItems.push(...items)
      hasMore = data?.data?.has_more || false
      page++
      if (page > 50) break // 安全上限
    }

    tracks.value = allItems
      .filter((item: any) => item.type === 2) // 仅视频
      .map((item: any) => ({
        id: `bilibili:${item.bvid || item.bv_id}`,
        title: item.title || '',
        artist: item.upper?.name || '',
        album: '',
        durationMs: (item.duration || 0) * 1000,
        coverUrl: item.cover || '',
        audioUrl: '',
      }))
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
          <span v-else class="material-symbols-rounded filled" style="font-size: 48px; opacity: 0.3">video_library</span>
        </div>
        <div class="hero-info">
          <h1 class="hero-title">{{ folderName }}</h1>
          <p class="hero-meta">{{ t('player.video_count', { count: mediaCount }) }}</p>
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
          <div class="track-cover-wide">
            <img v-if="track.coverUrl" :src="track.coverUrl" referrerpolicy="no-referrer" loading="lazy" @error="($event.target as HTMLImageElement).style.display = 'none'" />
            <span v-else class="material-symbols-rounded filled">movie</span>
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
