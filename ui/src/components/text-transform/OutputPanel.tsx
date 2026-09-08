import type { ReactNode } from "react"

export function OutputPanel({
  title,
  children,
  status,
}: {
  title: string
  children: ReactNode
  status?: ReactNode
}) {
  return (
    <section className="flex min-h-72 flex-col gap-2">
      <h2 className="text-sm font-medium">{title}</h2>
      <div className="flex min-h-0 flex-1 flex-col overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-sm">
        {children}
      </div>
      {status}
    </section>
  )
}
