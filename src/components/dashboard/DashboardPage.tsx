
import { LayoutDashboard } from "lucide-react";
import { EmptyState } from "@/components/shared/EmptyState";
import { useStats } from "@/hooks";
import { TrendChart } from "./TrendChart";
import { ProviderDistribution } from "./ProviderDistribution";
import { ModelDistribution } from "./ModelDistribution";

export function DashboardPage() {
  const { stats } = useStats();

  if (!stats) {
    return (
      <EmptyState
        icon={<LayoutDashboard className="h-12 w-12" />}
        title="仪表盘"
        description="加载统计数据中..."
      />
    );
  }

  return (
    <div className="flex flex-col gap-6 fade-in">
      <TrendChart data={stats.trend} />

      <div className="grid grid-cols-2 gap-4">
        <ProviderDistribution data={stats.provider_breakdown} />
        <ModelDistribution data={stats.model_breakdown} />
      </div>
    </div>
  );
}
