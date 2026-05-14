import { PanelLeftClose, PanelLeftOpen } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useUIStore } from '@renderer/stores/ui-store'
import { Button } from '@renderer/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@renderer/components/ui/tooltip'
import { cn } from '@renderer/lib/utils'

interface TitleBarProps {
  title: string
  subtitle?: string | null
  tooltip?: string | null
  showSidebarToggle?: boolean
  insetForMacTrafficLights?: boolean
}

export function TitleBar({
  title,
  tooltip = null,
  showSidebarToggle = true,
  insetForMacTrafficLights = false
}: TitleBarProps): React.JSX.Element {
  const { t } = useTranslation('layout')
  const isMac = /Mac/.test(navigator.userAgent)

  const leftSidebarOpen = useUIStore((s) => s.leftSidebarOpen)
  const toggleLeftSidebar = useUIStore((s) => s.toggleLeftSidebar)
  return (
    <header
      className={cn(
        'workspace-titlebar-surface titlebar-drag relative flex h-10 w-full shrink-0 items-center gap-3 overflow-hidden px-3',
        isMac && insetForMacTrafficLights ? 'pl-[78px]' : '',
        !isMac ? 'pr-[132px]' : ''
      )}
      style={{
        paddingRight: isMac ? undefined : 'calc(132px + 0.75rem)'
      }}
    >
      <div className="flex min-w-0 flex-1 items-center gap-3">
        {showSidebarToggle ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="workspace-titlebar-action titlebar-no-drag size-7 shrink-0 rounded-md text-muted-foreground hover:text-foreground"
                onClick={toggleLeftSidebar}
              >
                {leftSidebarOpen ? (
                  <PanelLeftClose className="size-4" />
                ) : (
                  <PanelLeftOpen className="size-4" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent>
              {t('commandPalette.toggleSidebar', { defaultValue: 'Toggle sidebar' })}
            </TooltipContent>
          </Tooltip>
        ) : null}

        <div className="min-w-0 flex-1">
          {title ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex min-w-0 items-center gap-2">
                  <div className="truncate text-sm font-semibold text-foreground/92">{title}</div>
                </div>
              </TooltipTrigger>
              {tooltip ? <TooltipContent>{tooltip}</TooltipContent> : null}
            </Tooltip>
          ) : null}
        </div>
      </div>
    </header>
  )
}
