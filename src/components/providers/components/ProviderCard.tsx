
import { Edit, Trash2, ShieldCheck, ShieldX } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Provider } from "@/lib/types";

interface ProviderCardProps {
  provider: Provider;
  togglingId: string | null;
  disableToggle: boolean;
  onToggle: (provider: Provider) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
}

export function ProviderCard({ provider, togglingId, disableToggle, onToggle, onEdit, onDelete }: ProviderCardProps) {
  return (
    <div className="signal-panel p-6 hover-lift">
      <div className="flex items-start justify-between gap-5">
        <div className="flex items-start gap-4">
          <div className={`mt-1 flex h-10 w-10 items-center justify-center ${
            provider.enabled ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground"
          }`}>
            {provider.enabled ? <ShieldCheck className="h-5 w-5" /> : <ShieldX className="h-5 w-5" />}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-3 flex-wrap">
              <span className="text-foreground font-semibold text-lg">{provider.name}</span>
              <Badge variant={provider.enabled ? "default" : "secondary"} className="text-xs">
                {provider.enabled ? "当前使用中" : "未启用"}
              </Badge>
              <Badge variant="outline" className="text-xs">
                {provider.protocol.toUpperCase()}
              </Badge>
            </div>
            <div className="text-muted-foreground text-sm font-mono mt-2">
              {provider.base_url}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Switch
            checked={provider.enabled}
            onChange={() => onToggle(provider)}
            disabled={disableToggle || togglingId !== null}
            aria-label={provider.enabled ? "Current provider" : "Enable provider"}
          />
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onEdit(provider)}
          >
            <Edit className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onDelete(provider)}
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </div>
      <div className="mt-4 flex flex-wrap gap-2">
        {provider.keyword && (
          <div className="bg-primary/5 border border-primary/20 px-4 py-2">
            <span className="text-muted-foreground text-xs font-mono">标签:</span>
            <span className="text-primary text-xs font-mono ml-1.5">{provider.keyword}</span>
          </div>
        )}
        {provider.model_mapping && (
          <div className="bg-muted border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs font-mono">映射:</span>
            <span className="text-foreground text-xs font-mono ml-1.5">{provider.model_mapping}</span>
          </div>
        )}
        <div className="bg-muted border border-border px-4 py-2">
          <span className="text-muted-foreground text-xs font-mono">认证:</span>
          <span className="text-foreground text-xs font-mono ml-1.5">{provider.auth_header}</span>
        </div>
      </div>
    </div>
  );
}
