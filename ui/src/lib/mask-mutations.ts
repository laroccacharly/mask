import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import {
  fetchVaultMappings,
  inspectOccupations,
  resetVault,
  transformText,
  type Finding,
  type TransformOperation,
} from "@/lib/mask-api"

export const vaultMappingsQueryKey = ["vault", "mappings"] as const

export type TransformPane =
  | { kind: "inspect"; findings: Finding[]; elapsedMilliseconds: number }
  | { kind: "transform"; output: string; elapsedMilliseconds: number }

const clientState = {
  enabled: false,
  staleTime: Infinity,
  gcTime: Infinity,
} as const

function transformInputKey(operation: TransformOperation) {
  return ["transform", operation, "input"] as const
}

function transformFindingsKey(operation: TransformOperation) {
  return ["transform", operation, "findings"] as const
}

function transformPaneKey(operation: TransformOperation) {
  return ["transform", operation, "pane"] as const
}

export function useTransformDraft(operation: TransformOperation) {
  const queryClient = useQueryClient()
  const inputQuery = useQuery({
    queryKey: transformInputKey(operation),
    queryFn: () => "",
    initialData: "",
    ...clientState,
  })
  const findingsQuery = useQuery({
    queryKey: transformFindingsKey(operation),
    queryFn: (): Finding[] | null => null,
    initialData: null,
    ...clientState,
  })
  const paneQuery = useQuery({
    queryKey: transformPaneKey(operation),
    queryFn: (): TransformPane | null => null,
    initialData: null,
    ...clientState,
  })

  return {
    input: inputQuery.data,
    findings: findingsQuery.data,
    pane: paneQuery.data,
    setInput: (input: string) => {
      queryClient.setQueryData(transformInputKey(operation), input)
    },
    clearResults: () => {
      queryClient.setQueryData(transformFindingsKey(operation), null)
      queryClient.setQueryData(transformPaneKey(operation), null)
    },
  }
}

export function useTransformText(operation: TransformOperation) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (input: { text: string; findings?: Finding[] }) => {
      const startedAt = performance.now()
      const output = await transformText(
        operation,
        input.text,
        input.findings ?? []
      )

      return {
        output,
        elapsedMilliseconds: Math.round(performance.now() - startedAt),
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData(transformPaneKey(operation), {
        kind: "transform",
        output: result.output,
        elapsedMilliseconds: result.elapsedMilliseconds,
      } satisfies TransformPane)
      if (operation === "encode") {
        void queryClient.invalidateQueries({ queryKey: vaultMappingsQueryKey })
      }
    },
  })
}

export function useInspectOccupations() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (input: string) => {
      const startedAt = performance.now()
      const findings = await inspectOccupations(input)

      return {
        findings,
        elapsedMilliseconds: Math.round(performance.now() - startedAt),
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData(transformFindingsKey("encode"), result.findings)
      queryClient.setQueryData(transformPaneKey("encode"), {
        kind: "inspect",
        findings: result.findings,
        elapsedMilliseconds: result.elapsedMilliseconds,
      } satisfies TransformPane)
    },
  })
}

export function useVaultMappings() {
  return useQuery({
    queryKey: vaultMappingsQueryKey,
    queryFn: fetchVaultMappings,
    staleTime: 0,
  })
}

export function useResetVault() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: resetVault,
    onSuccess: (mappings) => {
      queryClient.setQueryData(vaultMappingsQueryKey, mappings)
    },
  })
}
