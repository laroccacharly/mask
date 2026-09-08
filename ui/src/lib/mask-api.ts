export type TransformOperation = "encode" | "decode"

export type Finding = {
  kind: string
  evidence: string[]
}

export type VaultMapping = {
  token: string
  value: string
  class: string
}

type EncodeRequest = {
  text: string
  findings: Finding[]
}

type InspectResponse = {
  findings: Finding[]
}

type VaultMappingsResponse = {
  mappings: VaultMapping[]
}

async function readError(response: Response, fallback: string) {
  const text = await response.text()
  if (!response.ok) {
    throw new Error(text || fallback)
  }
  return text
}

export async function inspectOccupations(input: string) {
  const response = await fetch("/api/inspect/occupations", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: input,
  })
  const text = await readError(response, "Occupation inspection failed")
  const body = JSON.parse(text) as InspectResponse
  return body.findings
}

export async function fetchVaultMappings() {
  const response = await fetch("/api/vault")
  const text = await readError(response, "Failed to load vault")
  const body = JSON.parse(text) as VaultMappingsResponse
  return body.mappings
}

export async function resetVault() {
  const response = await fetch("/api/vault/reset", { method: "POST" })
  const text = await readError(response, "Failed to reset vault")
  const body = JSON.parse(text) as VaultMappingsResponse
  return body.mappings
}

export async function transformText(
  operation: TransformOperation,
  input: string,
  findings: Finding[] = []
) {
  const isEncode = operation === "encode"
  const response = await fetch(`/api/${operation}`, {
    method: "POST",
    headers: {
      "Content-Type": isEncode ? "application/json" : "text/plain",
    },
    body: isEncode
      ? JSON.stringify({ text: input, findings } satisfies EncodeRequest)
      : input,
  })
  return readError(
    response,
    `${operation === "encode" ? "Encoding" : "Decoding"} failed`
  )
}
