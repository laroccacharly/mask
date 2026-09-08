import type { ComponentType, ReactNode } from "react"
import { useState } from "react"
import {
  ArchiveIcon,
  BracesIcon,
  BracketsIcon,
  MenuIcon,
  XIcon,
} from "lucide-react"
import { NavLink, useLocation } from "react-router-dom"

import { ThemeToggle } from "@/components/ThemeToggle"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

const navigation: Array<{
  to: string
  label: string
  description: string
  icon: ComponentType<{ className?: string }>
}> = [
  {
    to: "/encode",
    label: "Encode",
    description: "Protect sensitive text",
    icon: BracesIcon,
  },
  {
    to: "/decode",
    label: "Decode",
    description: "Restore original text",
    icon: BracketsIcon,
  },
  {
    to: "/vault",
    label: "Vault",
    description: "Inspect stored mappings",
    icon: ArchiveIcon,
  },
]

function Navigation({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="flex flex-col gap-0.5 px-2" aria-label="Main navigation">
      {navigation.map(({ to, label, description, icon: Icon }) => (
        <NavLink
          key={to}
          to={to}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors",
              isActive
                ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground"
                : "text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            )
          }
        >
          <Icon className="size-4 shrink-0" aria-hidden="true" />
          <span className="min-w-0 flex-1">
            <span className="block leading-5">{label}</span>
            <span className="block truncate text-xs font-normal text-muted-foreground">
              {description}
            </span>
          </span>
        </NavLink>
      ))}
    </nav>
  )
}

function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <>
      <div className="flex h-16 items-center gap-3 border-b border-sidebar-border px-4">
        <div className="flex size-9 items-center justify-center rounded-lg border bg-white shadow-xs">
          <img src="/fox.svg" alt="" className="size-6" />
        </div>
        <div>
          <p className="text-sm font-semibold">Mask</p>
          <p className="text-xs text-muted-foreground">Privacy console</p>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto py-3">
        <Navigation onNavigate={onNavigate} />
      </div>
    </>
  )
}

export function AppShell({ children }: { children: ReactNode }) {
  const [mobileOpen, setMobileOpen] = useState(false)
  const { pathname } = useLocation()
  const pageTitle =
    navigation.find((item) => item.to === pathname)?.label ?? "Mask"

  return (
    <div className="flex min-h-svh bg-background">
      <a
        href="#main-content"
        className="fixed top-3 left-3 z-60 -translate-y-20 rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground shadow-md transition-transform focus-visible:translate-y-0"
      >
        Skip to content
      </a>
      <aside className="sticky top-0 hidden h-svh w-60 shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground md:flex">
        <SidebarContent />
      </aside>

      {mobileOpen ? (
        <div className="fixed inset-0 z-50 md:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-black/30 backdrop-blur-xs"
            aria-label="Close navigation"
            onClick={() => setMobileOpen(false)}
          />
          <aside className="absolute inset-y-0 left-0 flex w-72 max-w-[85vw] flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground shadow-xl">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute top-3 right-3 text-sidebar-foreground"
              aria-label="Close navigation"
              onClick={() => setMobileOpen(false)}
            >
              <XIcon aria-hidden="true" />
            </Button>
            <SidebarContent onNavigate={() => setMobileOpen(false)} />
          </aside>
        </div>
      ) : null}

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-40 flex h-12 items-center gap-3 border-b bg-background/80 px-3 backdrop-blur-xl sm:px-5">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="md:hidden"
            aria-label="Open navigation"
            onClick={() => setMobileOpen(true)}
          >
            <MenuIcon aria-hidden="true" />
          </Button>
          <p className="min-w-0 flex-1 truncate text-sm font-medium">
            {pageTitle}
          </p>
          <ThemeToggle />
        </header>
        <main
          id="main-content"
          className="flex flex-1 flex-col bg-muted/20 p-4 sm:p-6"
        >
          <div className="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-6">
            {children}
          </div>
        </main>
      </div>
    </div>
  )
}
