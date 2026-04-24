/**
 * Median-Cut 调色板提取器
 * 从专辑封面图提取 4 个角色色 + shader 偏移参数
 * 移植自 Android 端 HyperBackground.kt 的 Palette 取色逻辑
 */

type RGB = [number, number, number] // 0-255

interface ColorBucket {
  pixels: RGB[]
  average: RGB
  population: number
}

export interface PaletteResult {
  dominant: RGB
  lightVibrant: RGB
  muted: RGB
  darkMuted: RGB
  lightOffset: number
  saturateOffset: number
  /** 底色 RGB */
  accentBg: RGB
}

// --- 色彩空间工具 ---

function luminance(r: number, g: number, b: number): number {
  return 0.299 * r + 0.587 * g + 0.114 * b
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const nr = r / 255, ng = g / 255, nb = b / 255
  const max = Math.max(nr, ng, nb), min = Math.min(nr, ng, nb)
  const l = (max + min) / 2
  if (max === min) return [0, 0, l]
  const d = max - min
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
  let h = 0
  if (max === nr) h = ((ng - nb) / d + (ng < nb ? 6 : 0)) / 6
  else if (max === ng) h = ((nb - nr) / d + 2) / 6
  else h = ((nr - ng) / d + 4) / 6
  return [h, s, l]
}

// --- Median-Cut 核心 ---

function channelRange(pixels: RGB[], ch: 0 | 1 | 2): number {
  let min = 255, max = 0
  for (const p of pixels) {
    if (p[ch] < min) min = p[ch]
    if (p[ch] > max) max = p[ch]
  }
  return max - min
}

function widestChannel(pixels: RGB[]): 0 | 1 | 2 {
  const rr = channelRange(pixels, 0)
  const gr = channelRange(pixels, 1)
  const br = channelRange(pixels, 2)
  if (rr >= gr && rr >= br) return 0
  if (gr >= rr && gr >= br) return 1
  return 2
}

function averageColor(pixels: RGB[]): RGB {
  if (pixels.length === 0) return [128, 128, 128]
  let tr = 0, tg = 0, tb = 0
  for (const p of pixels) { tr += p[0]; tg += p[1]; tb += p[2] }
  const n = pixels.length
  return [Math.round(tr / n), Math.round(tg / n), Math.round(tb / n)]
}

function medianCut(pixels: RGB[], maxBuckets: number): ColorBucket[] {
  if (pixels.length === 0) return []

  const buckets: RGB[][] = [pixels]

  while (buckets.length < maxBuckets) {
    // 找到像素最多的 bucket 进行分割
    let maxIdx = 0, maxLen = 0
    for (let i = 0; i < buckets.length; i++) {
      if (buckets[i].length > maxLen) {
        maxLen = buckets[i].length
        maxIdx = i
      }
    }
    // 只有一个像素的 bucket 无法再分
    if (maxLen <= 1) break

    const bucket = buckets[maxIdx]
    const ch = widestChannel(bucket)
    bucket.sort((a, b) => a[ch] - b[ch])
    const mid = Math.floor(bucket.length / 2)

    buckets.splice(maxIdx, 1, bucket.slice(0, mid), bucket.slice(mid))
  }

  return buckets.map(b => ({
    pixels: b,
    average: averageColor(b),
    population: b.length,
  }))
}

// --- 角色色选取 ---

function pickRoleColors(buckets: ColorBucket[]): {
  dominant: RGB
  lightVibrant: RGB
  muted: RGB
  darkMuted: RGB
} {
  // 回退色：灰色（对齐 Android 的 0xFF808080）
  const GRAY: RGB = [128, 128, 128]

  if (buckets.length === 0) {
    return { dominant: GRAY, lightVibrant: GRAY, muted: GRAY, darkMuted: GRAY }
  }

  // 按人口降序排序，取 dominant
  const sorted = [...buckets].sort((a, b) => b.population - a.population)
  const dominant = sorted[0].average

  // 为每个 bucket 计算 HSL
  const withHsl = buckets.map(b => {
    const [h, s, l] = rgbToHsl(b.average[0], b.average[1], b.average[2])
    return { ...b, h, s, l }
  })

  // LightVibrant: 高饱和度 + 亮度 > 0.4
  const lightVibrantCandidates = withHsl
    .filter(b => b.l > 0.4 && b.s > 0.15)
    .sort((a, b) => (b.s * b.l) - (a.s * a.l))
  // 回退到最亮的 bucket 而非 dominant（避免暗图全坍缩）
  const brightestBucket = [...withHsl].sort((a, b) => b.l - a.l)[0]
  const lightVibrant = lightVibrantCandidates[0]?.average
    ?? (brightestBucket && brightestBucket.l > 0.15 ? brightestBucket.average : GRAY)

  // Muted: 中等饱和度 + 中亮度
  const mutedCandidates = withHsl
    .filter(b => b.s < 0.6 && b.l > 0.15 && b.l < 0.8)
    .sort((a, b) => a.s - b.s)
  const muted = mutedCandidates[0]?.average ?? dominant

  // DarkMuted: 低亮度
  const darkMutedCandidates = withHsl
    .filter(b => b.l < 0.4)
    .sort((a, b) => a.l - b.l)
  const darkMuted = darkMutedCandidates[0]?.average ?? dominant

  return { dominant, lightVibrant, muted, darkMuted }
}

// --- 主入口 ---

/**
 * 从 ImageData 提取调色板
 * @param imageData 64x64 降采样后的图像数据
 * @param maxColors 最大量化色数，默认 16
 * @param isDark 暗色模式，影响 lightOffset/saturateOffset 计算
 */
export function extractPalette(
  imageData: ImageData,
  maxColors = 16,
  isDark = true,
): PaletteResult {
  const { data, width, height } = imageData
  const pixels: RGB[] = []
  const allPixels: RGB[] = []

  for (let i = 0; i < width * height; i++) {
    const idx = i * 4
    const r = data[idx], g = data[idx + 1], b = data[idx + 2]
    allPixels.push([r, g, b])
    const lum = luminance(r, g, b)
    // 过滤极端黑白像素（保留灰度范围内的像素）
    if (lum < 5 || lum > 250) continue
    pixels.push([r, g, b])
  }

  // 如果过滤后像素不足（如黑白/灰度封面），回退使用全部像素
  const effectivePixels = pixels.length >= 50 ? pixels : allPixels

  const buckets = medianCut(effectivePixels, maxColors)
  const roles = pickRoleColors(buckets)

  // 计算 shader 偏移
  // luma 用 BT.709 (0.2126, 0.7152, 0.0722)
  const dLum = (0.2126 * roles.dominant[0] + 0.7152 * roles.dominant[1] + 0.0722 * roles.dominant[2]) / 255

  // lightOffset = isDark ? (-0.06 + 0.12*(luma-0.5)) : (0.08 + 0.10*(0.5-luma))
  const lightOffset = isDark
    ? (-0.06 + 0.12 * (dLum - 0.5))
    : (0.08 + 0.10 * (0.5 - dLum))

  // saturateOffset = isDark ? 0.24 : 0.16（固定值）
  const saturateOffset = isDark ? 0.24 : 0.16

  // 计算 AccentBackdrop 底色（对齐 Android adjustedAccentColorArgb）
  const accentBg = computeAccentBg(roles.dominant, isDark)

  return {
    ...roles,
    lightOffset: clamp(lightOffset, -0.12, 0.12),
    saturateOffset,
    accentBg,
  }
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t
}

function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v))
}

/**
 * 计算 AccentBackdrop 底色
 * 对齐 Android adjustedAccentColorArgb：
 * - 转 HSL -> 大幅降饱和 -> 强制暗亮度 -> 与中性色混合
 */
function computeAccentBg(dominant: RGB, isDark: boolean): RGB {
  const [h, s, l] = rgbToHsl(dominant[0], dominant[1], dominant[2])

  // 降饱和（暗色 0.38x 最多 0.30，亮色 0.32x 最多 0.24）
  const targetS = isDark
    ? Math.min(s * 0.38, 0.30)
    : Math.min(s * 0.32, 0.24)

  // 暗色下亮度限制在 0.22-0.30（比 Android 稍亮以补偿 scrim 叠加），亮色下 0.90
  const targetL = isDark
    ? clamp(l, 0.22, 0.30)
    : 0.90

  const adjusted = hslToRgb(h, targetS, targetL)

  // 与中性色混合（暗色 0xFF121212 混 22%，亮色白色混 28%）
  const neutral: RGB = isDark ? [18, 18, 18] : [255, 255, 255]
  const blend = isDark ? 0.22 : 0.28

  return [
    Math.round(adjusted[0] * (1 - blend) + neutral[0] * blend),
    Math.round(adjusted[1] * (1 - blend) + neutral[1] * blend),
    Math.round(adjusted[2] * (1 - blend) + neutral[2] * blend),
  ]
}

function hslToRgb(h: number, s: number, l: number): RGB {
  if (s === 0) {
    const v = Math.round(l * 255)
    return [v, v, v]
  }
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1/6) return p + (q - p) * 6 * t
    if (t < 1/2) return q
    if (t < 2/3) return p + (q - p) * (2/3 - t) * 6
    return p
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q
  return [
    Math.round(hue2rgb(p, q, h + 1/3) * 255),
    Math.round(hue2rgb(p, q, h) * 255),
    Math.round(hue2rgb(p, q, h - 1/3) * 255),
  ]
}
