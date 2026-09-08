import { Button } from "@/components/ui/button"

export function TransformInput({
  value,
  onChange,
  placeholder,
  onFillExample,
}: {
  value: string
  onChange: (value: string) => void
  placeholder: string
  onFillExample?: () => void
}) {
  return (
    <div className="flex min-h-72 flex-col gap-2">
      <div className="flex items-center justify-between gap-2">
        <label htmlFor="transform-input" className="text-sm font-medium">
          Input
        </label>
        {onFillExample ? (
          <Button
            type="button"
            variant="ghost"
            size="xs"
            onClick={onFillExample}
          >
            Use example
          </Button>
        ) : null}
      </div>
      <textarea
        id="transform-input"
        className="flex-1 resize-none rounded-md border bg-background p-3 font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
      />
    </div>
  )
}
