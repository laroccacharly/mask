import { OutputPanel } from "@/components/text-transform/OutputPanel"
import { segmentTokenText, tokenStylesFor } from "@/lib/token-highlight"
import { cn } from "@/lib/utils"

function TokenizedText({ text }: { text: string }) {
  const segments = segmentTokenText(text)
  const styles = tokenStylesFor(segments)

  return (
    <>
      {segments.map((segment, index) => {
        if (segment.kind === "text") {
          return <span key={`text:${index}`}>{segment.value}</span>
        }
        if (segment.kind === "token") {
          return (
            <span
              key={`token:${index}:${segment.value}`}
              title={segment.tokenClass}
              data-token-class={segment.tokenClass}
              className={cn(
                "rounded-sm px-0.5 py-px font-medium ring-1 ring-current/15",
                styles.get(segment.tokenClass)
              )}
            >
              {segment.value}
            </span>
          )
        }
        const exhaustive: never = segment
        return exhaustive
      })}
    </>
  )
}

export function TransformOutput({
  output,
  elapsedMilliseconds,
  completedLabel,
}: {
  output: string
  elapsedMilliseconds: number
  completedLabel: string
}) {
  return (
    <OutputPanel
      title="Output"
      status={
        <p className="text-sm text-muted-foreground" role="status">
          {completedLabel} in {elapsedMilliseconds} ms
        </p>
      }
    >
      <pre aria-label="Output" className="m-0 flex-1 whitespace-pre-wrap">
        <TokenizedText key={output} text={output} />
      </pre>
    </OutputPanel>
  )
}
