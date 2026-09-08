import { useState } from "react"
import { ArchiveIcon } from "lucide-react"

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Input } from "@/components/ui/input"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { useResetVault, useVaultMappings } from "@/lib/mask-mutations"

export function VaultPage() {
  const [filter, setFilter] = useState("")
  const [confirmOpen, setConfirmOpen] = useState(false)
  const mappingsQuery = useVaultMappings()
  const resetVault = useResetVault()
  const mappings = mappingsQuery.data ?? []
  const query = filter.trim().toLowerCase()
  const visible =
    query.length === 0
      ? mappings
      : mappings.filter((mapping) => {
          return (
            mapping.token.toLowerCase().includes(query) ||
            mapping.value.toLowerCase().includes(query) ||
            mapping.class.toLowerCase().includes(query)
          )
        })
  const error = mappingsQuery.error?.message || resetVault.error?.message

  return (
    <section className="flex flex-1 flex-col gap-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Vault</h1>
          <p className="text-sm text-muted-foreground">
            Tokens stored in the current snapshot and the values they restore
            to.
          </p>
        </div>
        <Button
          type="button"
          variant="destructive"
          disabled={resetVault.isPending}
          onClick={() => setConfirmOpen(true)}
        >
          Reset vault
        </Button>
        <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Reset the vault?</AlertDialogTitle>
              <AlertDialogDescription>
                This deletes the snapshot on disk and creates an empty session.
                Encoded tokens from this vault will no longer decode.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                disabled={resetVault.isPending}
                onClick={() => {
                  resetVault.mutate(undefined, {
                    onSuccess: () => {
                      setConfirmOpen(false)
                      setFilter("")
                    },
                  })
                }}
              >
                {resetVault.isPending ? "Resetting…" : "Reset vault"}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>

      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <Input
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          placeholder="Filter by token, value, or class"
          aria-label="Filter mappings"
          className="sm:max-w-sm"
        />
        <p className="text-sm text-muted-foreground">
          {mappingsQuery.isPending
            ? "Loading mappings…"
            : `${visible.length} of ${mappings.length} mapping${
                mappings.length === 1 ? "" : "s"
              }`}
        </p>
      </div>

      {error ? (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      ) : null}

      {mappingsQuery.isPending ? null : mappings.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <ArchiveIcon />
            </EmptyMedia>
            <EmptyTitle>Vault is empty</EmptyTitle>
            <EmptyDescription>
              Encode text to store token mappings in the snapshot.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : visible.length === 0 ? (
        <Empty className="border">
          <EmptyHeader>
            <EmptyTitle>No matching mappings</EmptyTitle>
            <EmptyDescription>
              Nothing in the vault matches that filter.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="overflow-hidden rounded-md border bg-background">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Token</TableHead>
                <TableHead>Value</TableHead>
                <TableHead>Class</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {visible.map((mapping) => (
                <TableRow
                  key={`${mapping.class}:${mapping.token}:${mapping.value}`}
                >
                  <TableCell className="font-mono text-sm">
                    {mapping.token}
                  </TableCell>
                  <TableCell className="max-w-xl font-mono text-sm break-all whitespace-normal">
                    {mapping.value}
                  </TableCell>
                  <TableCell>
                    <Badge variant="outline">{mapping.class}</Badge>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </section>
  )
}
