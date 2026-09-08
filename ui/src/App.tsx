import { Navigate, Route, Routes } from "react-router-dom"

import { AppShell } from "@/components/AppShell"
import { DecodePage } from "@/views/DecodePage"
import { EncodePage } from "@/views/EncodePage"
import { VaultPage } from "@/views/VaultPage"

export function App() {
  return (
    <AppShell>
      <Routes>
        <Route path="/" element={<Navigate to="/encode" replace />} />
        <Route path="/encode" element={<EncodePage />} />
        <Route path="/decode" element={<DecodePage />} />
        <Route path="/vault" element={<VaultPage />} />
        <Route path="*" element={<Navigate to="/encode" replace />} />
      </Routes>
    </AppShell>
  )
}

export default App
