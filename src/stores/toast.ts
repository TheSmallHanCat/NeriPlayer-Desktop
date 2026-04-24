import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface ToastMessage {
  id: number
  text: string
  type: 'success' | 'error' | 'info'
  duration: number
}

export interface NotificationEntry {
  id: number
  text: string
  type: 'success' | 'error' | 'info'
  timestamp: number
  read: boolean
}

const MAX_HISTORY = 100

let nextId = 0

export const useToastStore = defineStore('toast', () => {
  const messages = ref<ToastMessage[]>([])
  const history = ref<NotificationEntry[]>([])

  /** 未读通知数 */
  const unreadCount = computed(() => history.value.filter(n => !n.read).length)

  function show(text: string, type: ToastMessage['type'] = 'info', duration = 3000) {
    const id = nextId++
    messages.value.push({ id, text, type, duration })
    setTimeout(() => dismiss(id), duration)

    // 同步写入历史
    history.value.unshift({
      id,
      text,
      type,
      timestamp: Date.now(),
      read: false,
    })
    // 限制历史长度
    if (history.value.length > MAX_HISTORY) {
      history.value = history.value.slice(0, MAX_HISTORY)
    }
  }

  function success(text: string, duration = 3000) {
    show(text, 'success', duration)
  }

  function error(text: string, duration = 4000) {
    show(text, 'error', duration)
  }

  function dismiss(id: number) {
    const idx = messages.value.findIndex(m => m.id === id)
    if (idx !== -1) messages.value.splice(idx, 1)
  }

  /** 标记所有通知为已读 */
  function markAllRead() {
    for (const n of history.value) {
      n.read = true
    }
  }

  /** 清空历史 */
  function clearHistory() {
    history.value = []
  }

  return { messages, history, unreadCount, show, success, error, dismiss, markAllRead, clearHistory }
})
