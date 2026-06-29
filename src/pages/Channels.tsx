import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { StateView } from "@/components/StateView"
import { useConfig } from "@/hooks/useConfig"
import { toast } from "sonner"

export function Channels() {
  const { config, loading, error, reload, update, isDirty, save, reset } = useConfig()

  if (loading || !config) return <StateView loading={loading} error={error} onRetry={reload}>{null}</StateView>

  const updateChannel = <K extends keyof typeof config.channels>(
    channel: K,
    patch: Partial<typeof config.channels[K]>,
  ) => update((d) => { Object.assign(d.channels[channel], patch) })

  const handleSave = () => {
    save().then((ok) => { if (ok) toast.success("已保存"); else toast.error("保存失败") })
  }

  return (
    <div className="space-y-4">
      {isDirty && (
        <div className="flex items-center justify-between rounded-md border border-yellow-500/50 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-700 dark:text-yellow-400">
          <span>未保存的修改</span>
          <Button size="sm" variant="ghost" onClick={reset}>放弃修改</Button>
        </div>
      )}
      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>飞书</CardTitle>
          <Switch checked={config.channels.feishu.enabled} onCheckedChange={(v) => updateChannel("feishu", { enabled: v })} />
        </CardHeader>
        {config.channels.feishu.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.feishu.appId} onChange={(e) => updateChannel("feishu", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>App Secret</Label><Input type="password" value={config.channels.feishu.appSecret} onChange={(e) => updateChannel("feishu", { appSecret: e.target.value })} /></div>
            <div className="space-y-1"><Label>Verification Token</Label><Input value={config.channels.feishu.verificationToken} onChange={(e) => updateChannel("feishu", { verificationToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Webhook Port</Label><Input type="number" value={config.channels.feishu.webhookPort} onChange={(e) => updateChannel("feishu", { webhookPort: Number(e.target.value) })} /></div>
            <div className="space-y-1"><Label>Encrypt Key</Label><Input value={config.channels.feishu.encryptKey} onChange={(e) => updateChannel("feishu", { encryptKey: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>QQ</CardTitle>
          <Switch checked={config.channels.qq.enabled} onCheckedChange={(v) => updateChannel("qq", { enabled: v })} />
        </CardHeader>
        {config.channels.qq.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.qq.appId} onChange={(e) => updateChannel("qq", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>Secret</Label><Input type="password" value={config.channels.qq.secret} onChange={(e) => updateChannel("qq", { secret: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Telegram</CardTitle>
          <Switch checked={config.channels.telegram.enabled} onCheckedChange={(v) => updateChannel("telegram", { enabled: v })} />
        </CardHeader>
        {config.channels.telegram.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.telegram.botToken} onChange={(e) => updateChannel("telegram", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Chat IDs（逗号分隔）</Label><Input value={config.channels.telegram.allowedChatIds.join(",")} onChange={(e) => updateChannel("telegram", { allowedChatIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Discord</CardTitle>
          <Switch checked={config.channels.discord.enabled} onCheckedChange={(v) => updateChannel("discord", { enabled: v })} />
        </CardHeader>
        {config.channels.discord.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.discord.botToken} onChange={(e) => updateChannel("discord", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Channel IDs（逗号分隔）</Label><Input value={config.channels.discord.allowedChannelIds.join(",")} onChange={(e) => updateChannel("discord", { allowedChannelIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>微信</CardTitle>
          <Switch checked={config.channels.wechat.enabled} onCheckedChange={(v) => updateChannel("wechat", { enabled: v })} />
        </CardHeader>
        {config.channels.wechat.enabled && (
          <CardContent><p className="text-sm text-muted-foreground">微信使用扫码登录，启动后请在弹窗中扫码。</p></CardContent>
        )}
      </Card>

      <Button onClick={handleSave} disabled={!isDirty}>保存</Button>
    </div>
  )
}
