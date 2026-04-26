import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { usePlayerStore, type TrackInfo } from './player'
import { useSettingsStore } from './settings'
import { useToastStore } from './toast'
import i18n from '@/i18n'

export interface DownloadedTrack {
  id: string
  title: string
  artist: string
  album: string
  durationMs: number
  coverUrl: string | null
  source: string
  filePath: string
  fileSize: number
  downloadedAt: number
}

export const useDownloadStore = defineStore('download', () => {
  const downloads = ref<DownloadedTrack[]>([])
  const downloading = ref<Map<string, { status: string; progress?: number }>>(new Map())

  let eventsInitialized = false

  function initEvents() {
    if (eventsInitialized) return
    eventsInitialized = true

    listen<{ trackId: string; status: string; fileSize?: number; message?: string }>(
      'download-progress',
      (e) => {
        const { trackId, status, message } = e.payload
        const toast = useToastStore()

        if (status === 'start') {
          downloading.value = new Map(downloading.value.set(trackId, { status: 'downloading' }))
        } else if (status === 'complete') {
          downloading.value.delete(trackId)
          downloading.value = new Map(downloading.value)
          loadDownloads()
          toast.success((i18n.global as any).t('download.downloaded'))
        } else if (status === 'error') {
          downloading.value.delete(trackId)
          downloading.value = new Map(downloading.value)
          toast.error((i18n.global as any).t('download.download_failed') + (message ? `: ${message}` : ''))
        } else if (status === 'already_exists') {
          downloading.value.delete(trackId)
          downloading.value = new Map(downloading.value)
        }
      },
    )
  }

  async function loadDownloads() {
    try {
      const raw = await invoke<any[]>('list_downloads')
      downloads.value = (raw || []).map((t: any) => ({
        id: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album,
        durationMs: t.duration_ms,
        coverUrl: t.cover_url || null,
        source: t.source,
        filePath: t.file_path,
        fileSize: t.file_size,
        downloadedAt: t.downloaded_at,
      }))
    } catch (e) {
      console.error('Load downloads failed:', e)
    }
  }

  /**
   * 下载曲目：先解析音频 URL（按来源分支），再调用后端下载
   */
  async function downloadTrack(track: TrackInfo) {
    initEvents()
    const toast = useToastStore()

    if (isDownloaded(track.id)) {
      toast.success((i18n.global as any).t('download.downloaded'))
      return
    }

    if (downloading.value.has(track.id)) {
      return // 正在下载中
    }

    downloading.value = new Map(downloading.value.set(track.id, { status: 'resolving' }))
    toast.success((i18n.global as any).t('download.downloading'))

    try {
      let audioUrl = ''

      if (track.id.startsWith('netease:')) {
        const settings = useSettingsStore()
        const songId = parseInt(track.id.replace('netease:', ''))
        const result = await invoke<{ url: string | null }>('get_netease_song_url', {
          songId,
          quality: settings.neteaseQuality,
        })
        if (!result.url) throw new Error('No URL')
        audioUrl = result.url
      } else if (track.id.startsWith('bilibili:')) {
        const biliId = track.id.replace('bilibili:', '')
        const isAvid = /^\d+$/.test(biliId)
        const cidMatch = track.album?.match(/^Bilibili\|(\d+)/)
        const cid = cidMatch ? parseInt(cidMatch[1]) : undefined
        const result = await invoke<{ url: string }>('get_bili_audio_url', {
          bvid: isAvid ? '' : biliId,
          avid: isAvid ? parseInt(biliId) : null,
          cid: cid || null,
        })
        audioUrl = result.url
      } else if (track.id.startsWith('youtube:')) {
        const videoId = track.id.replace('youtube:', '')
        const streams = await invoke<{ url: string }[]>('get_youtube_audio_url', { videoId })
        if (!streams?.[0]?.url) throw new Error('No YouTube stream')
        audioUrl = streams[0].url
      } else {
        // 本地文件无需下载
        toast.error((i18n.global as any).t('player.not_available'))
        downloading.value.delete(track.id)
        downloading.value = new Map(downloading.value)
        return
      }

      // 确定来源
      const source = track.id.startsWith('netease:')
        ? 'netease'
        : track.id.startsWith('bilibili:')
          ? 'bilibili'
          : track.id.startsWith('youtube:')
            ? 'youtube'
            : 'local'

      await invoke('download_track', {
        url: audioUrl,
        trackId: track.id,
        title: track.title,
        artist: track.artist,
        album: track.album || '',
        durationMs: track.durationMs,
        coverUrl: track.coverUrl || null,
        source,
        downloadDir: useSettingsStore().downloadDir || null,
        nameTemplate: useSettingsStore().downloadNameTemplate || null,
      })
    } catch (e: any) {
      console.error('Download failed:', e)
      downloading.value.delete(track.id)
      downloading.value = new Map(downloading.value)
      const msg = typeof e === 'string' ? e : e?.message || String(e)
      if (!msg.includes('already downloaded')) {
        toast.error((i18n.global as any).t('download.download_failed') + `: ${msg}`)
      }
    }
  }

  async function deleteDownload(trackId: string) {
    try {
      await invoke('delete_download', { trackId })
      downloads.value = downloads.value.filter(t => t.id !== trackId)
      const toast = useToastStore()
      toast.success((i18n.global as any).t('download.deleted'))
    } catch (e) {
      console.error('Delete download failed:', e)
    }
  }

  function isDownloaded(trackId: string): boolean {
    return downloads.value.some(t => t.id === trackId)
  }

  function isDownloading(trackId: string): boolean {
    return downloading.value.has(trackId)
  }

  function getDownloadedTrack(trackId: string): DownloadedTrack | undefined {
    return downloads.value.find(t => t.id === trackId)
  }

  function cancelAllDownloads() {
    // 清除所有进行中的下载状态
    downloading.value = new Map()
    const toast = useToastStore()
    toast.success((i18n.global as any).t('download.downloaded'))
  }

  return {
    downloads,
    downloading,
    loadDownloads,
    downloadTrack,
    deleteDownload,
    isDownloaded,
    isDownloading,
    getDownloadedTrack,
    cancelAllDownloads,
    initEvents,
  }
})
