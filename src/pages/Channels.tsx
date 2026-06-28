import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Switch } from "@/components/ui/switch"
import { getConfig, saveConfig } from "@/lib/tauri"
import type { AppConfig } from "@/lib/types"
import { toast } from "sonner"

export function Channels() {
  const [config, setConfig] = useState<AppConfig | null>(null)
  useEffect(() => { getConfig().then(setConfig) }, [])
  if (!config) return <div>加载中...</div>

  const update = (channel: keyof AppConfig["channels"], patch: Partial<AppConfig["channels"][keyof AppConfig["channels"]]>) =>
    setConfig({ ...config, channels: { ...config.channels, [channel]: { ...config.channels[channel], ...patch } } })

  const save = () => saveConfig(config).then(() => toast.success("已保存")).catch(() => toast.error("保存失败"))

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>飞书</CardTitle>
          <Switch checked={config.channels.feishu.enabled} onCheckedChange={(v) => update("feishu", { enabled: v })} />
        </CardHeader>
        {config.channels.feishu.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.feishu.appId} onChange={(e) => update("feishu", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>App Secret</Label><Input type="password" value={config.channels.feishu.appSecret} onChange={(e) => update("feishu", { appSecret: e.target.value })} /></div>
            <div className="space-y-1"><Label>Verification Token</Label><Input value={config.channels.feishu.verificationToken} onChange={(e) => update("feishu", { verificationToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Webhook Port</Label><Input type="number" value={config.channels.feishu.webhookPort} onChange={(e) => update("feishu", { webhookPort: Number(e.target.value) })} /></div>
            <div className="space-y-1"><Label>Encrypt Key</Label><Input value={config.channels.feishu.encryptKey} onChange={(e) => update("feishu", { encryptKey: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>QQ</CardTitle>
          <Switch checked={config.channels.qq.enabled} onCheckedChange={(v) => update("qq", { enabled: v })} />
        </CardHeader>
        {config.channels.qq.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>App ID</Label><Input value={config.channels.qq.appId} onChange={(e) => update("qq", { appId: e.target.value })} /></div>
            <div className="space-y-1"><Label>Secret</Label><Input type="password" value={config.channels.qq.secret} onChange={(e) => update("qq", { secret: e.target.value })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Telegram</CardTitle>
          <Switch checked={config.channels.telegram.enabled} onCheckedChange={(v) => update("telegram", { enabled: v })} />
        </CardHeader>
        {config.channels.telegram.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.telegram.botToken} onChange={(e) => update("telegram", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Chat IDs（逗号分隔）</Label><Input value={config.channels.telegram.allowedChatIds.join(",")} onChange={(e) => update("telegram", { allowedChatIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>Discord</CardTitle>
          <Switch checked={config.channels.discord.enabled} onCheckedChange={(v) => update("discord", { enabled: v })} />
        </CardHeader>
        {config.channels.discord.enabled && (
          <CardContent className="space-y-3">
            <div className="space-y-1"><Label>Bot Token</Label><Input type="password" value={config.channels.discord.botToken} onChange={(e) => update("discord", { botToken: e.target.value })} /></div>
            <div className="space-y-1"><Label>Allowed Channel IDs（逗号分隔）</Label><Input value={config.channels.discord.allowedChannelIds.join(",")} onChange={(e) => update("discord", { allowedChannelIds: e.target.value.split(",").map((s) => s.trim()).filter(Boolean) })} /></div>
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between"><CardTitle>微信</CardTitle>
          <Switch checked={config.channels.wechat.enabled} onCheckedChange={(v) => update("wechat", { enabled: v })} />
        </CardHeader>
        {config.channels.wechat.enabled && (
          <CardContent><p className="text-sm text-muted-foreground">微信使用扫码登录，启动后请在弹窗中扫码。</p></CardContent>
        )}
      </Card>

      <Button onClick={save}>保存</Button>
    </div>
  )
}
