<script setup lang="ts">
/**
 * 封面模糊背景组件
 * 将封面图全屏铺底并应用高斯模糊 + 暗化遮罩
 * 作为 HyperBackground 的替代背景模式
 */
defineProps<{
  coverUrl: string
  blurAmount: number   // 模糊像素值
  darkenAlpha: number  // 暗化遮罩透明度 0-1
}>()
</script>

<template>
  <div class="cover-blur-bg">
    <img
      v-if="coverUrl"
      :src="coverUrl"
      referrerpolicy="no-referrer"
      class="cover-blur-img"
      :style="{ filter: `blur(${blurAmount}px)` }"
    />
    <div class="cover-blur-darken" :style="{ background: `rgba(0,0,0,${darkenAlpha})` }" />
  </div>
</template>

<style scoped>
.cover-blur-bg {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
}

.cover-blur-img {
  /* 超出边缘避免模糊白边 */
  width: calc(100% + 80px);
  height: calc(100% + 80px);
  margin: -40px;
  object-fit: cover;
  will-change: filter;
}

.cover-blur-darken {
  position: absolute;
  inset: 0;
}
</style>
