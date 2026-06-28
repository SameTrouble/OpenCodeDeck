import { LogView } from "@/components/LogView"

export function Logs() {
  return (
    <div className="space-y-2">
      <h2 className="text-lg font-semibold">日志</h2>
      <LogView height="600px" />
    </div>
  )
}
