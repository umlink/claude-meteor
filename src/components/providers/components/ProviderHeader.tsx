
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ProviderHeaderProps {
  providersCount: number;
  enabledCount: number;
  onAddClick: () => void;
}

export function ProviderHeader({ providersCount, enabledCount, onAddClick }: ProviderHeaderProps) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <p className="panel-header-label">配置管理</p>
        <h1 className="mt-1 text-2xl font-semibold tracking-[-0.04em] text-foreground">提供商</h1>
      </div>
      <div className="flex items-center gap-4">
        <div className="text-muted-foreground text-sm">
          {providersCount} 总计, {enabledCount} 当前使用中
        </div>
        <Button
          onClick={onAddClick}
          className="flex items-center gap-2"
        >
          <Plus className="h-4 w-4" />
          新建提供商
        </Button>
      </div>
    </div>
  );
}
