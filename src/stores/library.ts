import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { TrackInfo } from './player'

export const useLibraryStore = defineStore('library', () => {
  const tracks = ref<TrackInfo[]>([])
  const isScanning = ref(false)
  const scanError = ref<string | null>(null)
  const lastScanDir = ref<string | null>(null)

  async function scanDirectory(dir: string) {
    isScanning.value = true
    scanError.value = null

    try {
      const results = await invoke<any[]>('scan_music_directory', { dir })

      // 后端返回的字段名是 snake_case，映射到前端 camelCase
      tracks.value = results.map(t => ({
        id: t.id,
        title: t.title,
        artist: t.artist,
        album: t.album,
        durationMs: t.duration_ms,
        coverUrl: t.cover_url || '',
        audioUrl: t.url,
      }))

      lastScanDir.value = dir
      // 持久化扫描路径
      localStorage.setItem('neri:last_scan_dir', dir)
    } catch (e: any) {
      scanError.value = String(e)
      console.error('Scan failed:', e)
    } finally {
      isScanning.value = false
    }
  }

  // 启动时恢复上次扫描路径
  function restoreLastScan() {
    const dir = localStorage.getItem('neri:last_scan_dir')
    if (dir) scanDirectory(dir)
  }

  return {
    tracks, isScanning, scanError, lastScanDir,
    scanDirectory, restoreLastScan,
  }
})
