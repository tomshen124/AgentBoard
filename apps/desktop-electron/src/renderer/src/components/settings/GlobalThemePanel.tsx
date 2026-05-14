import { Monitor, MoonStar, SunMedium } from 'lucide-react'
import { useTheme } from 'next-themes'
import { useTranslation } from 'react-i18next'
import { cn } from '@renderer/lib/utils'
import { useSettingsStore } from '@renderer/stores/settings-store'

const MODE_OPTIONS = [
  {
    value: 'light',
    icon: SunMedium,
    labelKey: 'general.light'
  },
  {
    value: 'dark',
    icon: MoonStar,
    labelKey: 'general.dark'
  },
  {
    value: 'system',
    icon: Monitor,
    labelKey: 'general.system'
  }
] as const

export function GlobalThemePanel({
  className
}: {
  compact?: boolean
  className?: string
}): React.JSX.Element {
  const { t } = useTranslation('settings')
  const { setTheme } = useTheme()
  const settings = useSettingsStore()

  return (
    <div className={cn('space-y-5', className)}>
      <section className="space-y-3">
        <div>
          <div className="text-sm font-medium text-foreground">{t('general.theme')}</div>
          <p className="text-xs text-muted-foreground">{t('general.themeDesc')}</p>
        </div>

        <div className="grid grid-cols-3 gap-2">
          {MODE_OPTIONS.map((option) => {
            const active = settings.theme === option.value
            const Icon = option.icon

            return (
              <button
                key={option.value}
                type="button"
                onClick={() => {
                  settings.updateSettings({ theme: option.value })
                  setTheme(option.value)
                }}
                className={cn(
                  'flex items-center justify-center gap-2 rounded-[16px] border px-3 py-3 text-sm transition-all',
                  active
                    ? 'border-primary bg-primary text-primary-foreground shadow-[0_16px_32px_-24px_color-mix(in_srgb,var(--primary)_75%,transparent)]'
                    : 'border-border bg-card text-foreground hover:border-foreground/15 hover:bg-accent'
                )}
              >
                <Icon className="size-4" />
                <span>{t(option.labelKey)}</span>
              </button>
            )
          })}
        </div>
      </section>
    </div>
  )
}
