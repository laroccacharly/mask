import type { Finding } from "@/lib/mask-api"

import { OutputPanel } from "@/components/text-transform/OutputPanel"

function evidenceCount(findings: Finding[]) {
  return findings.reduce((total, finding) => total + finding.evidence.length, 0)
}

export function InspectOccupationsOutput({
  findings,
  elapsedMilliseconds,
}: {
  findings: Finding[]
  elapsedMilliseconds: number
}) {
  const spans = evidenceCount(findings)

  return (
    <OutputPanel
      title="Occupations"
      status={
        <p className="text-sm text-muted-foreground" role="status">
          Inspected occupations in {elapsedMilliseconds} ms
        </p>
      }
    >
      <div aria-label="Output">
        <p className="font-sans text-sm font-medium">
          Occupations ready: {spans} span{spans === 1 ? "" : "s"}
        </p>
        {findings.length === 0 ? (
          <p className="mt-1 font-sans text-sm text-muted-foreground">
            No occupation findings. Encode will use the standard detectors.
          </p>
        ) : (
          <ul className="mt-2 list-disc space-y-1 pl-5">
            {findings.flatMap((finding) =>
              finding.evidence.map((span, index) => (
                <li key={`${finding.kind}:${index}:${span}`}>{span}</li>
              ))
            )}
          </ul>
        )}
      </div>
    </OutputPanel>
  )
}
