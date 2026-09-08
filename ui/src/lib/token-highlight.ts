export type TokenSegment =
  | { kind: "text"; value: string }
  | { kind: "token"; value: string; tokenClass: string }

const TOKEN_PATTERN = /<(Custom:[a-z0-9_]*|[A-Za-z][A-Za-z0-9]*)_\d+>/gi

const TOKEN_CLASS_STYLES = [
  "bg-sky-500/20 text-sky-800 dark:text-sky-200",
  "bg-amber-500/20 text-amber-800 dark:text-amber-200",
  "bg-violet-500/20 text-violet-800 dark:text-violet-200",
  "bg-emerald-500/20 text-emerald-800 dark:text-emerald-200",
  "bg-rose-500/20 text-rose-800 dark:text-rose-200",
  "bg-cyan-500/20 text-cyan-800 dark:text-cyan-200",
  "bg-orange-500/20 text-orange-800 dark:text-orange-200",
  "bg-fuchsia-500/20 text-fuchsia-800 dark:text-fuchsia-200",
  "bg-lime-500/20 text-lime-800 dark:text-lime-200",
  "bg-indigo-500/20 text-indigo-800 dark:text-indigo-200",
] as const

const KNOWN_TOKEN_CLASSES: Record<string, number> = {
  Name: 0,
  Organization: 1,
  Location: 2,
  Email: 3,
  "Custom:occupation": 4,
}

export function segmentTokenText(text: string): TokenSegment[] {
  const pattern = new RegExp(TOKEN_PATTERN.source, TOKEN_PATTERN.flags)
  const segments: TokenSegment[] = []
  let cursor = 0

  for (const match of text.matchAll(pattern)) {
    const start = match.index ?? 0
    if (start > cursor) {
      segments.push({ kind: "text", value: text.slice(cursor, start) })
    }
    segments.push({
      kind: "token",
      value: match[0],
      tokenClass: normalizeTokenClass(match[1]),
    })
    cursor = start + match[0].length
  }

  if (cursor < text.length) {
    segments.push({ kind: "text", value: text.slice(cursor) })
  }

  return segments
}

export function tokenStylesFor(segments: TokenSegment[]): Map<string, string> {
  const classes = [
    ...new Set(
      segments
        .filter((segment) => segment.kind === "token")
        .map((segment) => segment.tokenClass)
    ),
  ]
  const styles = new Map<string, string>()
  const used = new Set<number>()

  for (const tokenClass of classes) {
    const known = KNOWN_TOKEN_CLASSES[tokenClass]
    if (known === undefined || used.has(known)) {
      continue
    }
    styles.set(tokenClass, TOKEN_CLASS_STYLES[known])
    used.add(known)
  }

  for (const tokenClass of classes) {
    if (styles.has(tokenClass)) {
      continue
    }
    const next = TOKEN_CLASS_STYLES.findIndex((_, index) => !used.has(index))
    const index = next === -1 ? paletteIndex(tokenClass) : next
    styles.set(tokenClass, TOKEN_CLASS_STYLES[index])
    used.add(index)
  }

  return styles
}

function normalizeTokenClass(tokenClass: string): string {
  const custom = /^custom:(.*)$/i.exec(tokenClass)
  if (!custom) {
    return tokenClass
  }
  return `Custom:${custom[1].toLowerCase()}`
}

function paletteIndex(tokenClass: string): number {
  let hash = 0
  for (const character of tokenClass) {
    hash = (hash * 33 + character.charCodeAt(0)) | 0
  }
  return Math.abs(hash) % TOKEN_CLASS_STYLES.length
}
