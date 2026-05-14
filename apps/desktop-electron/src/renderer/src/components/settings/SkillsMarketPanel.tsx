import type { ReactNode } from 'react'
import {
  CloudCog,
  ExternalLink,
  Github,
  KeyRound,
  PackageCheck,
  ShieldCheck,
  Store,
  Wand2
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@renderer/components/ui/button'
import { Input } from '@renderer/components/ui/input'
import { Separator } from '@renderer/components/ui/separator'
import { useUIStore } from '@renderer/stores/ui-store'
import { useSettingsStore, type SkillsMarketSource } from '@renderer/stores/settings-store'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@renderer/components/ui/select'

const SKILL_SOURCE_LINKS = {
  skillhub: 'https://skillhub.cn/',
  clawhub: 'https://clawhub.ai/',
  githubSearch: 'https://github.com/search?q=filename%3ASKILL.md&type=code'
} as const

const SKILL_SOURCE_OPTIONS: Array<{ value: SkillsMarketSource; label: string }> = [
  { value: 'clawhub', label: 'ClawHub' },
  { value: 'skillhub', label: 'SkillHub' },
  { value: 'github', label: 'GitHub' }
]

interface SourceCard {
  key: string
  icon: ReactNode
  accent: string
  href?: string
  action?: () => void
}

export function SkillsMarketPanel(): React.JSX.Element {
  const { t } = useTranslation('settings')
  const openSettingsPage = useUIStore((s) => s.openSettingsPage)
  const skillsMarketSource = useSettingsStore((s) => s.skillsMarketSource)
  const skillsMarketApiKey = useSettingsStore((s) => s.skillsMarketApiKey)
  const updateSettings = useSettingsStore((s) => s.updateSettings)

  const sourceCards: SourceCard[] = [
    {
      key: 'builtin',
      icon: <PackageCheck className="size-4" />,
      accent: 'text-emerald-500'
    },
    {
      key: 'github',
      icon: <Github className="size-4" />,
      accent: 'text-foreground',
      href: SKILL_SOURCE_LINKS.githubSearch
    },
    {
      key: 'skillhub',
      icon: <Store className="size-4" />,
      accent: 'text-blue-500',
      href: SKILL_SOURCE_LINKS.skillhub
    },
    {
      key: 'clawhub',
      icon: <Wand2 className="size-4" />,
      accent: 'text-violet-500',
      href: SKILL_SOURCE_LINKS.clawhub
    },
    {
      key: 'connectors',
      icon: <CloudCog className="size-4" />,
      accent: 'text-sky-500',
      action: () => openSettingsPage('mcp')
    },
    {
      key: 'review',
      icon: <ShieldCheck className="size-4" />,
      accent: 'text-amber-500'
    }
  ]

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-lg font-semibold">{t('skillsmarket.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('skillsmarket.subtitle')}</p>
      </div>

      <Separator />

      <section className="space-y-3 rounded-lg border border-border/60 bg-muted/30 p-4">
        <div>
          <label className="text-sm font-medium">{t('skillsmarket.source')}</label>
          <p className="text-xs text-muted-foreground">{t('skillsmarket.sourceDesc')}</p>
        </div>
        <Select
          value={skillsMarketSource}
          onValueChange={(value) =>
            updateSettings({ skillsMarketSource: value as SkillsMarketSource })
          }
        >
          <SelectTrigger className="max-w-sm text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SKILL_SOURCE_OPTIONS.map((source) => (
              <SelectItem key={source.value} value={source.value} className="text-xs">
                {source.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <div className="max-w-sm space-y-1.5">
          <label className="text-sm font-medium">{t('skillsmarket.apiKey')}</label>
          <Input
            type="password"
            value={skillsMarketApiKey}
            onChange={(e) => updateSettings({ skillsMarketApiKey: e.target.value })}
            placeholder={t('skillsmarket.apiKeyPlaceholder')}
            className="text-xs"
          />
          <p className="text-xs leading-5 text-muted-foreground">{t('skillsmarket.apiKeyDesc')}</p>
        </div>
      </section>

      <Separator />

      <section className="space-y-3">
        <div className="grid gap-3 md:grid-cols-2">
          {sourceCards.map((source) => {
            const clickable = source.href || source.action

            return (
              <div
                key={source.key}
                className="flex min-h-[132px] flex-col rounded-lg border border-border/60 bg-muted/25 p-4"
              >
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <span className={source.accent}>{source.icon}</span>
                  {t(`skillsmarket.sources.${source.key}.title`)}
                </div>
                <p className="flex-1 text-xs leading-5 text-muted-foreground">
                  {t(`skillsmarket.sources.${source.key}.desc`)}
                </p>
                {clickable && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-3 h-7 w-fit gap-1.5 text-xs"
                    onClick={() => {
                      if (source.href) window.open(source.href, '_blank', 'noopener')
                      source.action?.()
                    }}
                  >
                    {t(`skillsmarket.sources.${source.key}.action`)}
                    <ExternalLink className="size-3" />
                  </Button>
                )}
              </div>
            )
          })}
        </div>
      </section>

      <Separator />

      <section className="space-y-3 rounded-lg border border-border/60 bg-muted/30 p-4">
        <div className="flex items-center gap-2 text-sm font-medium">
          <KeyRound className="size-4 text-primary" />
          {t('skillsmarket.keyPolicy.title')}
        </div>
        <div className="grid gap-2 text-xs leading-5 text-muted-foreground md:grid-cols-2">
          <p>{t('skillsmarket.keyPolicy.builtin')}</p>
          <p>{t('skillsmarket.keyPolicy.github')}</p>
          <p>{t('skillsmarket.keyPolicy.external')}</p>
          <p>{t('skillsmarket.keyPolicy.connectors')}</p>
        </div>
      </section>

      <section className="space-y-3 rounded-lg border border-border/60 bg-background p-4">
        <div className="flex items-center gap-2 text-sm font-medium">
          <ShieldCheck className="size-4 text-amber-500" />
          {t('skillsmarket.installPolicy.title')}
        </div>
        <p className="text-xs leading-5 text-muted-foreground">
          {t('skillsmarket.installPolicy.desc')}
        </p>
      </section>
    </div>
  )
}
