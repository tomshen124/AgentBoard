import * as React from 'react'
import {
  FileText,
  ImageIcon,
  Code2,
  PanelRightClose,
  PanelRightOpen,
  Settings,
  MessageSquare,
  Bot,
  Wrench,
  BookOpen,
  UserCircle,
  Target
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@renderer/components/ui/button'
import { InputArea } from '@renderer/components/chat/InputArea'
import { ProjectTerminalDock } from '@renderer/components/terminal/ProjectTerminalDock'
import { WorkingFolderSelectorDialog } from '@renderer/components/chat/WorkingFolderSelectorDialog'
import { useUIStore } from '@renderer/stores/ui-store'
import { useChatStore } from '@renderer/stores/chat-store'
import { useChatActions, type SendMessageOptions } from '@renderer/hooks/use-chat-actions'
import type { ImageAttachment } from '@renderer/lib/image-attachments'
import { PreviewPanel } from '@renderer/components/layout/PreviewPanel'
import { cn } from '@renderer/lib/utils'

const RENDER_PANEL_MIN_WIDTH = 300

export function ProjectHomePage(): React.JSX.Element {
  const { t } = useTranslation('chat')
  const activeProjectId = useChatStore((state) => state.activeProjectId)
  const projects = useChatStore((state) => state.projects)
  const mode = useUIStore((state) => state.mode)
  const previewPanelOpen = useUIStore((state) => state.previewPanelOpen)
  const previewPanelTabs = useUIStore((state) => state.previewPanelTabs)
  const activePreviewPanelTabId = useUIStore((state) => state.activePreviewPanelTabId)
  const setActivePreviewTab = useUIStore((state) => state.setActivePreviewTab)

  const resolvedProject =
    projects.find((project) => project.id === activeProjectId) ??
    projects.find((project) => !project.pluginId) ??
    projects[0] ??
    null
  const terminalDockOpen = useUIStore((state) =>
    resolvedProject?.id
      ? Boolean(state.bottomTerminalDockOpenByProjectId[resolvedProject.id])
      : false
  )
  const activeProject = resolvedProject
  const workingFolder = activeProject?.workingFolder
  const sshConnectionId = activeProject?.sshConnectionId
  const allSessions = useChatStore((state) => state.sessions)
  const navigateToSession = useUIStore((state) => state.navigateToSession)
  const navigateToArchive = useUIStore((state) => state.navigateToArchive)
  const { sendMessage } = useChatActions()
  const [folderDialogOpen, setFolderDialogOpen] = React.useState(false)

  const recentSessions = React.useMemo(() => {
    if (!activeProject?.id) return []
    return allSessions
      .filter((s) => s.projectId === activeProject.id)
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .slice(0, 5)
  }, [allSessions, activeProject?.id])

  const openSession = React.useCallback(
    (sessionId: string) => {
      if (activeProject?.id) {
        useChatStore.getState().setActiveProject(activeProject.id)
      }
      navigateToSession(sessionId)
    },
    [activeProject?.id, navigateToSession]
  )

  const contractFiles = [
    { id: 'AGENTS.md', icon: <Bot className="size-3.5" />, label: 'AGENTS.md' },
    { id: 'TOOLS.md', icon: <Wrench className="size-3.5" />, label: 'TOOLS.md' },
    { id: 'MEMORY.md', icon: <BookOpen className="size-3.5" />, label: 'MEMORY.md' },
    { id: 'PROFILE.md', icon: <UserCircle className="size-3.5" />, label: 'PROFILE.md' },
    { id: 'FOCUS.md', icon: <Target className="size-3.5" />, label: 'FOCUS.md' }
  ]

  const formatRelativeTime = (updatedAt: number): string => {
    const elapsed = Date.now() - updatedAt
    const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })
    const minutes = Math.floor(elapsed / 60000)
    if (minutes < 1) return 'just now'
    if (minutes < 60) return rtf.format(-minutes, 'minute')
    const hours = Math.floor(minutes / 60)
    if (hours < 24) return rtf.format(-hours, 'hour')
    const days = Math.floor(hours / 24)
    if (days < 30) return rtf.format(-days, 'day')
    return new Date(updatedAt).toLocaleDateString()
  }

  const handleSend = React.useCallback(
    (text: string, images?: ImageAttachment[], options?: SendMessageOptions): void => {
      const chatStore = useChatStore.getState()
      const projectId = activeProject?.id ?? activeProjectId
      if (!projectId) return
      chatStore.setActiveProject(projectId)
      const sessionId = chatStore.createSession(mode, projectId)
      useUIStore.getState().navigateToSession(sessionId)
      void sendMessage(text, images, undefined, sessionId, undefined, undefined, {
        ...options,
        clearCompletedTasksOnTurnStart: true
      })
    },
    [activeProject?.id, activeProjectId, mode, sendMessage]
  )

  const updateProjectDirectory = React.useCallback(
    async (patch: { workingFolder: string; sshConnectionId: string | null }): Promise<void> => {
      const projectId = activeProject?.id ?? activeProjectId
      if (!projectId) return
      useChatStore.getState().setActiveProject(projectId)
      useChatStore.getState().updateProjectDirectory(projectId, patch)
    },
    [activeProject?.id, activeProjectId]
  )

  const toggleRenderPanel = React.useCallback(() => {
    const store = useUIStore.getState()
    if (store.previewPanelOpen) {
      store.closePreviewPanel()
    } else {
      useUIStore.setState({ previewPanelOpen: true })
    }
  }, [])

  if (!activeProject) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center bg-background px-6">
        <div className="w-full max-w-[520px] text-center">
          <p className="text-[28px] font-semibold tracking-tight text-foreground">
            {t('projectHome.noProjectSelected')}
          </p>
          <p className="mt-3 text-sm leading-6 text-muted-foreground">
            {t('projectHome.noProjectSelectedDesc')}
          </p>
          <Button
            className="mt-6 h-9 rounded-md px-4"
            onClick={() => useUIStore.getState().navigateToHome()}
          >
            {t('projectHome.backHome')}
          </Button>
        </div>
      </div>
    )
  }

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden bg-background">
      {/* Main workspace */}
      <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden">
        <div className="flex flex-1 flex-col items-center overflow-auto px-7 pb-12 pt-8">
          <div className="flex w-full max-w-[720px] flex-col gap-5">
            <header className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <h1 className="text-[30px] font-semibold tracking-tight text-foreground">
                  {activeProject.name}
                </h1>
                <p className="mt-1 text-sm text-muted-foreground">
                  {t('projectHome.workspaceSubtitle', {
                    defaultValue: '在这个项目中提问、构建、迭代。'
                  })}
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="mt-1 h-9 shrink-0 gap-2 rounded-md text-xs"
                onClick={() => setFolderDialogOpen(true)}
              >
                <Settings className="size-3.5" />
                {t('projectHome.projectSettings', { defaultValue: 'Project Settings' })}
              </Button>
            </header>

            <div className="h-5" />

            {/* Recent Threads */}
            {recentSessions.length > 0 && (
              <section>
                <h3 className="mb-2 text-xs font-medium tracking-wider text-muted-foreground uppercase">
                  {t('projectHome.recentThreads', { defaultValue: 'Recent Threads' })}
                </h3>
                <div className="-mx-2 space-y-0.5">
                  {recentSessions.map((session) => (
                    <button
                      key={session.id}
                      onClick={() => openSession(session.id)}
                      className="flex h-8 w-full items-center gap-2.5 rounded-lg px-2 text-left text-[13px] text-muted-foreground transition-colors hover:bg-muted/60 hover:text-foreground"
                    >
                      <MessageSquare className="size-3.5 shrink-0" />
                      <span className="min-w-0 flex-1 truncate">{session.title || 'Untitled'}</span>
                      <span className="shrink-0 text-[11px] text-muted-foreground/60">
                        {session.messageCount}
                      </span>
                      <span className="shrink-0 text-[11px] text-muted-foreground/50">
                        {formatRelativeTime(session.updatedAt)}
                      </span>
                    </button>
                  ))}
                </div>
              </section>
            )}

            {/* Project Contracts */}
            <section>
              <h3 className="mb-2 text-xs font-medium tracking-wider text-muted-foreground uppercase">
                {t('projectHome.projectContracts', { defaultValue: 'Project Contracts' })}
              </h3>
              <div className="flex flex-wrap gap-1.5">
                {contractFiles.map((file) => (
                  <button
                    key={file.id}
                    onClick={() => activeProject?.id && navigateToArchive(activeProject.id)}
                    className="flex h-7 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-[12px] text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
                  >
                    {file.icon}
                    <span>{file.label}</span>
                  </button>
                ))}
              </div>
            </section>

            <div className="h-5" />

            {/* Composer */}
            <section>
              <p className="mb-3 text-sm text-muted-foreground">
                {t('projectHome.heroDesc', {
                  defaultValue: 'Start a task or ask a question for this project.'
                })}
              </p>
              <div className="rounded-xl border border-border/70 bg-background shadow-xs">
                <InputArea
                  sessionId={null}
                  onSend={handleSend}
                  onSelectFolder={() => setFolderDialogOpen(true)}
                  workingFolder={workingFolder}
                  hideWorkingFolderIndicator
                  hideWorkingFolderPicker
                  isStreaming={false}
                />
              </div>
            </section>
          </div>
        </div>

        {terminalDockOpen && (workingFolder || sshConnectionId) && (
          <ProjectTerminalDock
            projectId={activeProject.id}
            projectName={activeProject.name}
            workingFolder={workingFolder ?? null}
            sshConnectionId={sshConnectionId}
          />
        )}

        {/*
          Toggle button sits on the seam between main workspace and render panel.
          Its parent has position: relative, so absolute positioning works.
        */}
        {!previewPanelOpen && (
          <button
            onClick={toggleRenderPanel}
            className="absolute right-0 top-1/2 z-10 -translate-y-1/2 translate-x-1/2 rounded-full border border-border/60 bg-background p-1.5 text-muted-foreground shadow-sm hover:text-foreground"
            title={t('renderPanel.open', { defaultValue: '打开渲染区' })}
          >
            <PanelRightOpen className="size-3.5" />
          </button>
        )}
      </div>

      {/* Right: Render Panel */}
      <div
        className={cn(
          'flex shrink-0 flex-col border-l border-border/60 bg-background transition-all duration-200',
          previewPanelOpen ? 'opacity-100' : 'w-0 overflow-hidden border-transparent opacity-0'
        )}
        style={{
          width: previewPanelOpen ? RENDER_PANEL_MIN_WIDTH : 0,
          minWidth: previewPanelOpen ? RENDER_PANEL_MIN_WIDTH : 0
        }}
      >
        <div className="flex items-center justify-between border-b border-border/60 px-3 py-2">
          <span className="text-xs font-medium text-muted-foreground">
            {t('renderPanel.title', { defaultValue: '渲染区' })}
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="size-6"
            onClick={toggleRenderPanel}
            title={t('renderPanel.close', { defaultValue: '关闭渲染区' })}
          >
            <PanelRightClose className="size-3.5" />
          </Button>
        </div>

        {previewPanelTabs.length > 0 && (
          <div className="flex items-center gap-0.5 border-b border-border/40 px-2 py-1">
            {previewPanelTabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActivePreviewTab(tab.id)}
                className={cn(
                  'flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors',
                  tab.id === activePreviewPanelTabId
                    ? 'bg-muted text-foreground'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {tab.filePath.endsWith('.md') ? (
                  <FileText className="size-3" />
                ) : /\.(png|jpg|jpeg|gif|svg)$/i.test(tab.filePath) ? (
                  <ImageIcon className="size-3" />
                ) : (
                  <Code2 className="size-3" />
                )}
                <span className="max-w-[100px] truncate">
                  {tab.title || tab.filePath.split('/').pop()}
                </span>
              </button>
            ))}
          </div>
        )}

        <div className="min-h-0 flex-1 overflow-hidden">
          <PreviewPanel embedded />
        </div>
      </div>

      <WorkingFolderSelectorDialog
        open={folderDialogOpen}
        onOpenChange={setFolderDialogOpen}
        workingFolder={workingFolder}
        sshConnectionId={sshConnectionId}
        onSelectLocalFolder={(folderPath) =>
          updateProjectDirectory({
            workingFolder: folderPath,
            sshConnectionId: null
          })
        }
        onSelectSshFolder={(folderPath, connectionId) =>
          updateProjectDirectory({
            workingFolder: folderPath,
            sshConnectionId: connectionId
          })
        }
      />
    </div>
  )
}
