import {
  Brain,
  CalendarDays,
  Cable,
  CircleHelp,
  FileText,
  Folder,
  Search,
  Settings,
  Shapes,
  Sparkles,
  SquarePen,
  Wand2
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Tooltip, TooltipContent, TooltipTrigger } from '@renderer/components/ui/tooltip'
import { Popover, PopoverContent, PopoverTrigger } from '@renderer/components/ui/popover'
import { useUIStore, type NavItem } from '@renderer/stores/ui-store'
import { useResourcesStore } from '@renderer/stores/resources-store'
import { cn } from '@renderer/lib/utils'
import packageJson from '../../../../../package.json'

interface NavItemDef {
  value: NavItem | 'new-thread' | 'search'
  icon: React.ReactNode
  labelKey: string
}

interface StudioSubItemDef {
  value: string
  icon: React.ReactNode
  labelKey: string
}

const topNavItems: NavItemDef[] = [
  { value: 'new-thread', icon: <SquarePen className="size-5" />, labelKey: 'navRail.newThread' },
  { value: 'search', icon: <Search className="size-5" />, labelKey: 'navRail.search' },
  { value: 'projects', icon: <Folder className="size-5" />, labelKey: 'navRail.projects' },
  { value: 'studio', icon: <Shapes className="size-5" />, labelKey: 'navRail.studio' },
  { value: 'connections', icon: <Cable className="size-5" />, labelKey: 'navRail.connections' },
  {
    value: 'automations',
    icon: <CalendarDays className="size-5" />,
    labelKey: 'navRail.automations'
  }
]

const studioSubItems: StudioSubItemDef[] = [
  { value: 'agents', icon: <Brain className="size-4" />, labelKey: 'navRail.agents' },
  { value: 'commands', icon: <Wand2 className="size-4" />, labelKey: 'navRail.commands' },
  { value: 'skills', icon: <Sparkles className="size-4" />, labelKey: 'navRail.skills' },
  { value: 'canvas', icon: <FileText className="size-4" />, labelKey: 'navRail.canvas' }
]

export function NavRail(): React.JSX.Element {
  const { t } = useTranslation('layout')
  const activeNavItem = useUIStore((s) => s.activeNavItem)
  const setActiveNavItem = useUIStore((s) => s.setActiveNavItem)
  const leftSidebarOpen = useUIStore((s) => s.leftSidebarOpen)
  const studioPageOpen = useUIStore((s) => s.studioPageOpen)
  const connectionsPageOpen = useUIStore((s) => s.connectionsPageOpen)
  const tasksPageOpen = useUIStore((s) => s.tasksPageOpen)

  const handleNavClick = (item: string): void => {
    const ui = useUIStore.getState()

    // Special items
    if (item === 'new-thread') {
      ui.navigateToHome()
      return
    }

    if (item === 'search') {
      if (leftSidebarOpen && activeNavItem === 'threads') {
        const input = document.querySelector<HTMLInputElement>('[data-sidebar-search]')
        input?.focus()
      } else {
        setActiveNavItem('threads')
        if (!leftSidebarOpen) ui.setLeftSidebarOpen(true)
        requestAnimationFrame(() => {
          const input = document.querySelector<HTMLInputElement>('[data-sidebar-search]')
          input?.focus()
        })
      }
      return
    }

    if (item === 'projects') {
      setActiveNavItem('projects')
      if (!leftSidebarOpen) ui.setLeftSidebarOpen(true)
      return
    }

    if (item === 'connections') {
      if (connectionsPageOpen) {
        ui.closeConnectionsPage()
        return
      }
      ui.openConnectionsPage()
      return
    }

    if (item === 'automations') {
      if (tasksPageOpen) {
        ui.closeTasksPage()
        return
      }
      ui.openTasksPage()
      return
    }

    // Default nav toggle
    if (activeNavItem === item && leftSidebarOpen) {
      ui.setLeftSidebarOpen(false)
    } else {
      setActiveNavItem(item as NavItem)
      if (!leftSidebarOpen) ui.setLeftSidebarOpen(true)
    }
  }

  const handleStudioSubClick = (subItem: string): void => {
    const ui = useUIStore.getState()
    switch (subItem) {
      case 'agents':
        useResourcesStore.getState().setActiveKind('agents')
        ui.openResourcesPage()
        break
      case 'commands':
        useResourcesStore.getState().setActiveKind('commands')
        ui.openResourcesPage()
        break
      case 'skills':
        ui.openSkillsPage()
        break
      case 'canvas':
        ui.openDrawPage()
        break
    }
  }

  const studioActive =
    studioPageOpen ||
    useUIStore.getState().resourcesPageOpen ||
    useUIStore.getState().skillsPageOpen ||
    useUIStore.getState().drawPageOpen

  const isActive = (value: string): boolean => {
    switch (value) {
      case 'new-thread':
      case 'search':
        return false
      case 'studio':
        return studioActive
      case 'connections':
        return connectionsPageOpen
      case 'automations':
        return tasksPageOpen
      default:
        return activeNavItem === value && leftSidebarOpen
    }
  }

  return (
    <div className="flex h-full w-12 shrink-0 flex-col items-center border-r bg-muted/30 py-2">
      {/* Top nav items */}
      <div className="flex flex-col items-center gap-1">
        {topNavItems.map((item) =>
          item.value === 'studio' ? (
            <Popover key={item.value}>
              <Tooltip>
                <TooltipTrigger asChild>
                  <PopoverTrigger asChild>
                    <button
                      className={cn(
                        'flex size-9 items-center justify-center rounded-lg transition-all duration-200',
                        isActive(item.value)
                          ? 'bg-primary/10 text-primary shadow-sm'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      )}
                    >
                      {item.icon}
                    </button>
                  </PopoverTrigger>
                </TooltipTrigger>
                <TooltipContent side="right">{t(item.labelKey)}</TooltipContent>
              </Tooltip>
              <PopoverContent side="right" align="start" className="w-44 p-1.5">
                <div className="flex flex-col gap-0.5">
                  {studioSubItems.map((sub) => (
                    <button
                      key={sub.value}
                      onClick={() => handleStudioSubClick(sub.value)}
                      className="flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted hover:text-foreground"
                    >
                      {sub.icon}
                      <span>{t(sub.labelKey)}</span>
                    </button>
                  ))}
                </div>
              </PopoverContent>
            </Popover>
          ) : (
            <Tooltip key={item.value}>
              <TooltipTrigger asChild>
                <button
                  onClick={() => handleNavClick(item.value)}
                  className={cn(
                    'flex size-9 items-center justify-center rounded-lg transition-all duration-200',
                    isActive(item.value)
                      ? 'bg-primary/10 text-primary shadow-sm'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  )}
                >
                  {item.icon}
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">{t(item.labelKey)}</TooltipContent>
            </Tooltip>
          )
        )}
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Bottom: Help, Settings, Version */}
      <div className="flex flex-col items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={() => {
                /* Help action — could open docs */
              }}
              className="flex size-9 items-center justify-center rounded-lg text-muted-foreground transition-all duration-200 hover:bg-muted hover:text-foreground"
            >
              <CircleHelp className="size-5" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{t('navRail.help')}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={() => useUIStore.getState().openSettingsPage()}
              className="flex size-9 items-center justify-center rounded-lg text-muted-foreground transition-all duration-200 hover:bg-muted hover:text-foreground"
            >
              <Settings className="size-5" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{t('navRail.settings')}</TooltipContent>
        </Tooltip>
        <span className="text-[9px] text-muted-foreground/40 select-none">
          v{packageJson.version}
        </span>
      </div>
    </div>
  )
}
