<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()

const navItems = [
  { path: '/', icon: 'home', key: 'nav.home' },
  { path: '/explore', icon: 'explore', key: 'nav.explore' },
  { path: '/library', icon: 'library_music', key: 'nav.library' },
  { path: '/settings', icon: 'settings', key: 'nav.settings' },
]
</script>

<template>
  <nav class="side-nav">
    <!-- 应用图标 -->
    <div class="nav-logo">
      <img src="/app-icon.png" alt="NeriPlayer" class="app-icon" />
    </div>

    <div class="nav-list">
      <div
        v-for="item in navItems"
        :key="item.path"
        class="nav-item"
        :class="{ active: route.path === item.path }"
        @click="router.push(item.path)"
      >
        <div class="nav-pill">
          <span class="material-symbols-rounded" :class="{ filled: route.path === item.path }">
            {{ item.icon }}
          </span>
        </div>
        <span class="nav-label">{{ t(item.key) }}</span>
      </div>
    </div>
  </nav>
</template>

<style scoped lang="scss">
.side-nav {
  width: 80px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--md-surface-container-low);
  border-right: 1px solid var(--md-surface-container-high);
  flex-shrink: 0;
  padding: 0 0 12px;
}

.nav-logo {
  width: 100%;
  height: 52px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  /* 允许拖拽标题栏区域 */
  -webkit-app-region: drag;
}

.app-icon {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  object-fit: contain;
}

.nav-list {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding-top: 4px;
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  padding: 4px 0;
  transition: transform var(--duration-short) var(--ease-standard);

  &:active { transform: scale(0.92); }

  &:hover .nav-pill {
    background: var(--md-surface-container-highest);
  }

  &.active .nav-pill {
    background: var(--md-primary-container);
    color: var(--md-on-primary-container);
  }

  &.active .nav-label {
    font-weight: 700;
    color: var(--md-on-surface);
  }
}

.nav-pill {
  width: 56px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-xl);
  color: var(--md-on-surface-variant);
  transition: background var(--duration-short) var(--ease-standard),
              color var(--duration-short) var(--ease-standard);
}

.nav-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--md-on-surface-variant);
  transition: all var(--duration-short);
}
</style>
