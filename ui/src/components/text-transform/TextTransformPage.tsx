import { useState, type ReactNode } from "react"

import { InspectOccupationsOutput } from "@/components/text-transform/InspectOccupationsOutput"
import { TransformInput } from "@/components/text-transform/TransformInput"
import { TransformOutput } from "@/components/text-transform/TransformOutput"
import { Button } from "@/components/ui/button"
import type { TransformOperation } from "@/lib/mask-api"
import {
  useInspectOccupations,
  useTransformDraft,
  useTransformText,
  type TransformPane,
} from "@/lib/mask-mutations"

const EXAMPLE_INPUT = `Weekly clinic follow-up, 4 April 2026
Attendees: Jonah Hale, Elena Park, warehouse leads from Cleveland.

The overtime report is still the main cost driver. Jonah asked for a six-week diagnostic
and a steering readout on 24 April. Send the Affiliate a copy. Do not email the customer.
Jonah Hale is a labor economist.

Elena joined late because of weather in Boise. She walked through bed capacity, on-call
coverage, and the stalled Northwind acquisition. Dr. Elena Park is a pediatric neurosurgeon.
The group agreed to keep Project Cedar off the written agenda until legal reviews the draft.

Action items: Hale will send the readout template. Park will confirm next week's room.
`

const content = {
  encode: {
    title: "Encode",
    description: "Replace sensitive information with secure tokens.",
    action: "Encode",
    working: "Encoding…",
    inputPlaceholder: "Enter text to encode",
    emptyError: "Input cannot be empty",
    completed: "Encoded",
  },
  decode: {
    title: "Decode",
    description: "Restore secure tokens to their original values.",
    action: "Decode",
    working: "Decoding…",
    inputPlaceholder: "Enter text to decode",
    emptyError: "Input cannot be empty",
    completed: "Decoded",
  },
}

function renderRightPane(
  pane: TransformPane | null,
  completedLabel: string
): ReactNode {
  if (!pane) {
    return null
  }

  switch (pane.kind) {
    case "inspect":
      return (
        <InspectOccupationsOutput
          findings={pane.findings}
          elapsedMilliseconds={pane.elapsedMilliseconds}
        />
      )
    case "transform":
      return (
        <TransformOutput
          output={pane.output}
          elapsedMilliseconds={pane.elapsedMilliseconds}
          completedLabel={completedLabel}
        />
      )
    default: {
      const exhaustive: never = pane
      return exhaustive
    }
  }
}

export function TextTransformPage({
  operation,
}: {
  operation: TransformOperation
}) {
  const { input, findings, pane, setInput, clearResults } =
    useTransformDraft(operation)
  const [validationError, setValidationError] = useState("")
  const mutation = useTransformText(operation)
  const inspectOccupations = useInspectOccupations()
  const labels = content[operation]
  const canInspectOccupations = operation === "encode"
  const busy = mutation.isPending || inspectOccupations.isPending

  function clearOutput() {
    clearResults()
    mutation.reset()
    inspectOccupations.reset()
  }

  function submit() {
    setValidationError("")

    if (input.trim().length === 0) {
      setValidationError(labels.emptyError)
      return
    }

    mutation.mutate({ text: input, findings: findings ?? [] })
  }

  function inspectOccupationsText() {
    setValidationError("")

    if (input.trim().length === 0) {
      setValidationError(labels.emptyError)
      return
    }

    inspectOccupations.mutate(input)
  }

  const error =
    validationError ||
    mutation.error?.message ||
    inspectOccupations.error?.message

  return (
    <section className="flex flex-1 flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold">{labels.title}</h1>
        <p className="text-sm text-muted-foreground">{labels.description}</p>
      </div>

      <div
        className={
          pane ? "grid flex-1 gap-4 md:grid-cols-2" : "grid flex-1 gap-4"
        }
      >
        <TransformInput
          value={input}
          onChange={(value) => {
            setInput(value)
            clearOutput()
          }}
          placeholder={labels.inputPlaceholder}
          onFillExample={
            canInspectOccupations
              ? () => {
                  setInput(EXAMPLE_INPUT)
                  setValidationError("")
                  clearOutput()
                }
              : undefined
          }
        />
        {renderRightPane(pane, labels.completed)}
      </div>

      <div className="flex items-center gap-4">
        {canInspectOccupations ? (
          <Button
            variant="outline"
            onClick={inspectOccupationsText}
            disabled={busy}
          >
            {inspectOccupations.isPending
              ? "Inspecting occupations…"
              : "Inspect occupations"}
          </Button>
        ) : null}
        <Button onClick={submit} disabled={busy}>
          {mutation.isPending ? labels.working : labels.action}
        </Button>
        {error ? (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        ) : null}
      </div>
    </section>
  )
}
