/** The person's local day (`YYYY-MM-DD`) and IANA time zone, for the agent prompt. */
export interface LocalDay {
  today: string
  timezone: string
}

export function localDay(now: Date = new Date()): LocalDay {
  const pad = (n: number) => String(n).padStart(2, '0')
  const today = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
  let timezone = 'UTC'
  try {
    timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
  } catch {
    // Keep UTC: the Rust side validates the pair and falls back anyway.
  }
  return { today, timezone }
}
