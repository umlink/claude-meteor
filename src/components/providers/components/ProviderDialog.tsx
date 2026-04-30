
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import type { Provider } from "@/lib/types";

interface ProviderForm {
  name: string;
  base_url: string;
  api_key: string;
  protocol: "anthropic" | "openai";
  model_mapping: string;
  auth_header: "x-api-key" | "bearer";
  keyword: string;
  enabled: boolean;
}

interface ProviderDialogProps {
  editProvider: Provider | null;
  form: ProviderForm;
  onChange: (v: ProviderForm) => void;
  onClose: () => void;
  onSave: () => void;
  open: boolean;
  canSave: boolean;
}

const keywordOptions = ["opus", "sonnet", "haiku"];

export function ProviderDialog({
  editProvider,
  form,
  onChange,
  onClose,
  onSave,
  open,
  canSave,
}: ProviderDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(nextOpen) => { if (!nextOpen) onClose(); }}>
      <DialogContent className="signal-panel overflow-x-hidden">
        <DialogHeader>
          <DialogTitle className="text-left font-semibold text-xl">
            {editProvider ? "编辑提供商" : "新建提供商"}
          </DialogTitle>
        </DialogHeader>
        <div className="mt-2 flex min-w-0 flex-col gap-5">
          <div className="grid min-w-0 grid-cols-1 gap-5 sm:grid-cols-2">
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="name" className="text-sm font-medium">
                名称
              </Label>
              <Input
                id="name"
                required
                placeholder="例如：DeepSeek"
                value={form.name}
                onChange={(e) => onChange({ ...form, name: e.target.value })}
              />
            </div>
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="keyword" className="text-sm font-medium">
                标签分组
              </Label>
              <Select
                value={form.keyword}
                onValueChange={(keyword) => {
                  if (!keyword) return;
                  onChange({ ...form, keyword });
                }}
              >
                <SelectTrigger id="keyword" className="w-full min-w-0">
                  <SelectValue placeholder="选择标签分组" />
                </SelectTrigger>
                <SelectContent>
                  {keywordOptions.map((keyword) => (
                    <SelectItem key={keyword} value={keyword} className="text-sm">
                      {keyword}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-muted-foreground text-xs">
                仅用于列表分组展示，不参与 `/model` 路由。
              </p>
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="base_url" className="text-sm font-medium">
              基础 URL
            </Label>
            <Input
              id="base_url"
              required
              placeholder="例如：https://api.example.com"
              value={form.base_url}
              onChange={(e) => onChange({ ...form, base_url: e.target.value })}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="api_key" className="text-sm font-medium">
              API 密钥
            </Label>
            <Input
              id="api_key"
              type="password"
              required={!editProvider}
              value={form.api_key}
              onChange={(e) => onChange({ ...form, api_key: e.target.value })}
              placeholder={editProvider ? "留空则保持不变" : "请输入 API 密钥"}
            />
          </div>
          <div className="grid min-w-0 grid-cols-1 gap-5 sm:grid-cols-2">
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="protocol" className="text-sm font-medium">
                协议
              </Label>
              <Select
                value={form.protocol}
                onValueChange={(v) => onChange({ ...form, protocol: v as "anthropic" | "openai" })}
              >
                <SelectTrigger id="protocol" className="w-full min-w-0" disabled>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="anthropic" className="text-sm">Anthropic</SelectItem>
                  <SelectItem value="openai" className="text-sm">OpenAI</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="auth_header" className="text-sm font-medium">
                认证头
              </Label>
              <Select
                value={form.auth_header}
                onValueChange={(v) => onChange({ ...form, auth_header: v as "x-api-key" | "bearer" })}
              >
                <SelectTrigger id="auth_header" className="w-full min-w-0" disabled>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="x-api-key" className="text-sm">X-Api-Key</SelectItem>
                  <SelectItem value="bearer" className="text-sm">Bearer</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="model_mapping" className="text-sm font-medium">
              模型映射
            </Label>
            <Input
              id="model_mapping"
              value={form.model_mapping}
              onChange={(e) => onChange({ ...form, model_mapping: e.target.value })}
              placeholder="例如：gpt-4.1 / claude-sonnet-4-5（可选）"
            />
            <p className="text-muted-foreground text-xs">
              当前启用哪个提供商，请求就会直接走哪个提供商；这里仅指定上游实际模型名。
            </p>
          </div>
        </div>
        <DialogFooter className="flex gap-3 mt-6">
          <Button onClick={onClose} variant="outline" className="flex-1">
            取消
          </Button>
          <Button onClick={onSave} disabled={!canSave} className="flex-1">
            保存
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
