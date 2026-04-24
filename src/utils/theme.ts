// 深色/浅色模式管理 + 圆形扩散过渡动画
import { applyThemeColor, getSavedThemeColor } from './themeColor'

export type ThemeMode = 'system' | 'dark' | 'light'

let currentMode: ThemeMode = 'system'

export function getThemeMode(): ThemeMode {
  return (localStorage.getItem('theme-mode') as ThemeMode) || 'system'
}

function resolvedIsDark(mode: ThemeMode): boolean {
  if (mode === 'dark') return true
  if (mode === 'light') return false
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

/** 无动画直接应用 */
export function applyTheme(mode: ThemeMode) {
  currentMode = mode
  localStorage.setItem('theme-mode', mode)
  const dark = resolvedIsDark(mode)
  document.documentElement.classList.toggle('light-theme', !dark)
  document.documentElement.classList.toggle('dark-theme', dark)
  // 重新应用主题色的 light/dark 变量集
  applyThemeColor(getSavedThemeColor())
}

/**
 * 带圆形扩散动画的主题切换
 * @param mode 目标模式
 * @param x 点击位置 x
 * @param y 点击位置 y
 */
export async function switchThemeWithRipple(mode: ThemeMode, x: number, y: number) {
  // 如果浏览器不支持 View Transition，直接切换
  if (!(document as any).startViewTransition) {
    applyTheme(mode)
    return
  }

  // 计算圆形遮罩最大半径（覆盖整个屏幕）
  const maxRadius = Math.hypot(
    Math.max(x, window.innerWidth - x),
    Math.max(y, window.innerHeight - y),
  )

  const transition = (document as any).startViewTransition(() => {
    applyTheme(mode)
  })

  try {
    await transition.ready

    // 新视图从 clip-path: circle(0) 扩散到 circle(maxRadius)
    document.documentElement.animate(
      {
        clipPath: [
          `circle(0px at ${x}px ${y}px)`,
          `circle(${maxRadius}px at ${x}px ${y}px)`,
        ],
      },
      {
        duration: 800,
        easing: 'cubic-bezier(0.2, 0, 0, 1)',
        pseudoElement: '::view-transition-new(root)',
      },
    )
  } catch {
    // transition 被中断也没关系
  }
}

// 初始化
export function initTheme() {
  const mode = getThemeMode()
  applyTheme(mode)
  // 初始化主题色
  applyThemeColor(getSavedThemeColor())

  // 监听系统主题变化
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (getThemeMode() === 'system') {
      applyTheme('system')
    }
  })
}
