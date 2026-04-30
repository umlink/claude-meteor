import { NavLink } from "react-router-dom";
import { useState } from "react";
import {
  Activity,
  AlertTriangle,
  Clock,
  LayoutDashboard,
  Network,
  Play,
  ScrollText,
  Settings,
  Square,
  Zap,
} from "lucide-react";
import {
  startProxy,
  stopProxy,
  injectClaudeConfig,
  revertClaudeConfig,
} from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import { useProxyStatus, useStats } from "@/hooks";
import { toast } from "sonner";

const navItems = [
  { to: "/dashboard", label: "仪表盘", icon: LayoutDashboard },
  { to: "/providers", label: "提供商", icon: Network },
  { to: "/logs", label: "日志", icon: ScrollText },
  { to: "/settings", label: "设置", icon: Settings },
];

function LogoMark({ className = "" }: { className?: string }) {
  return (
    <img
      src="/favicon.png"
      alt="Meteor"
      className={`h-12 w-12 flex-shrink-0 rounded-lg object-contain ${className}`}
    />
  );
}

const formatMetric = (value: number | undefined, suffix = "") => {
  if (value === undefined || Number.isNaN(value)) return "--";
  return `${new Intl.NumberFormat("zh-CN", { notation: "compact" }).format(value)}${suffix}`;
};

export function TopNav() {
  const { status, refresh: refreshStatus } = useProxyStatus();
  const { stats } = useStats();
  const [isToggling, setIsToggling] = useState(false);

  const handleToggle = async () => {
    if (isToggling) return;
    setIsToggling(true);
    try {
      if (status?.running) {
        await Promise.all([stopProxy(), revertClaudeConfig()]);
        toast.success("代理已停止，Claude 配置已还原");
      } else {
        await Promise.all([startProxy(), injectClaudeConfig()]);
        toast.success("代理已启动，Claude 配置已应用");
      }
      await refreshStatus();
    } catch (error) {
      toast.error(`操作失败: ${error}`);
    } finally {
      setIsToggling(false);
    }
  };

  const today = stats?.today;
  const totalTokens =
    today === undefined ? undefined : today.total_input_tokens + today.total_output_tokens;
  const metrics = [
    {
      label: "今日请求",
      value: formatMetric(today?.total_requests),
      icon: Activity,
    },
    {
      label: "今日错误",
      value: formatMetric(today?.total_errors),
      icon: AlertTriangle,
      tone: today?.total_errors ? "text-red-600" : "text-slate-950",
    },
    {
      label: "Token",
      value: formatMetric(totalTokens),
      icon: Zap,
    },
    {
      label: "延迟",
      value: formatMetric(
        today === undefined ? undefined : Math.round(today.avg_latency_ms),
        today === undefined ? "" : "ms",
      ),
      icon: Clock,
    },
  ];

  return (
    <>
      <aside className="fixed inset-y-0 left-0 z-50 hidden w-[72px] flex-col border-y border-r border-slate-200 bg-white/95 backdrop-blur-xl lg:flex">
        <div className="flex h-[73px] items-center justify-center border-b border-slate-200">
          <LogoMark />
        </div>

        <nav className="flex flex-1 flex-col items-stretch">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              title={label}
              className={({ isActive }) =>
                `nav-pill group relative flex h-[72px] flex-col items-center justify-center gap-1 border-b border-slate-200 text-[11px] font-semibold transition-colors ${
                  isActive
                    ? "bg-slate-50 text-slate-950"
                    : "text-slate-500 hover:bg-slate-50 hover:text-slate-950"
                }`
              }
            >
              {({ isActive }) => (
                <>
                  <span
                    className={`absolute left-0 top-0 h-full w-0.5 bg-primary transition-opacity ${
                      isActive ? "opacity-100" : "opacity-0"
                    }`}
                  />
                  <Icon className="h-[18px] w-[18px] flex-shrink-0" />
                  <span className="max-w-[56px] truncate">{label}</span>
                </>
              )}
            </NavLink>
          ))}
        </nav>
      </aside>

      <header className="sticky top-0 z-40 border-y border-slate-200 bg-white/95 backdrop-blur-xl">
        <div className="mx-auto max-w-7xl px-4 sm:px-6">
          <div className="grid min-h-[72px] grid-cols-[minmax(0,1fr)_auto] items-stretch lg:grid-cols-[220px_minmax(0,1fr)_280px] xl:grid-cols-[260px_minmax(0,1fr)_280px]">
            <div className="flex min-w-0 items-center gap-3 border-r border-slate-200 pr-4 sm:pr-6">
              <LogoMark className="lg:hidden" />
              <div className="min-w-0">
                <div className="truncate text-base font-bold tracking-normal text-slate-950">
                  METEOR
                </div>
                <div className="truncate text-xs font-medium text-slate-500">
                  代理网关控制台
                </div>
              </div>
            </div>

            <div className="hidden min-w-0 grid-cols-4 border-r border-slate-200 lg:grid">
              {metrics.map(({ label, value, icon: Icon, tone }) => (
                <div
                  key={label}
                  className="flex min-w-0 items-center gap-2 border-r border-slate-200 px-3 last:border-r-0 xl:px-4"
                >
                  <Icon className="h-4 w-4 flex-shrink-0 text-slate-400" />
                  <div className="min-w-0">
                    <div className="truncate text-[11px] font-semibold text-slate-500">
                      {label}
                    </div>
                    <div
                      className={`truncate font-mono text-sm font-bold ${
                        tone ?? "text-slate-950"
                      }`}
                    >
                      {value}
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <div className="flex w-auto items-center justify-end gap-3 py-3 pl-4 sm:pl-6 lg:w-[280px]">
              <div className="hidden w-[168px] items-center justify-end gap-2 sm:flex">
                <span
                  className={`h-2 w-2 flex-shrink-0 rounded-full ${
                    status?.running ? "bg-primary status-dot" : "bg-slate-300"
                  }`}
                />
                <div
                  className={`w-14 text-right text-xs font-bold tracking-wide ${
                    status?.running ? "text-emerald-700" : "text-slate-400"
                  }`}
                >
                  {status?.running ? "ONLINE" : "OFFLINE"}
                </div>
                <div className="w-20 text-right font-mono text-xs font-medium text-slate-500">
                  {status?.port ? `:${status.port}` : ":----"}
                </div>
              </div>

              <Button
                className="h-9 w-20 gap-1.5 rounded-md px-0 text-sm font-semibold"
                variant={status?.running ? "destructive" : "default"}
                onClick={handleToggle}
                disabled={isToggling}
              >
                {status?.running ? (
                  <>
                    <Square className="h-3.5 w-3.5" /> 停止
                  </>
                ) : (
                  <>
                    <Play className="h-3.5 w-3.5" /> 启动
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>
      </header>

      <nav className="fixed inset-x-0 bottom-0 z-50 grid grid-cols-4 border-t border-slate-200 bg-white/95 backdrop-blur-xl lg:hidden">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `nav-pill relative flex h-16 min-w-0 flex-col items-center justify-center gap-1 border-r border-slate-200 text-[11px] font-semibold last:border-r-0 ${
                isActive
                  ? "bg-slate-50 text-slate-950"
                  : "text-slate-500 hover:text-slate-950"
              }`
            }
          >
            {({ isActive }) => (
              <>
                <span
                  className={`absolute inset-x-0 top-0 h-0.5 bg-primary transition-opacity ${
                    isActive ? "opacity-100" : "opacity-0"
                  }`}
                />
                <Icon className="h-4 w-4 flex-shrink-0" />
                <span className="max-w-[72px] truncate">{label}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>
    </>
  );
}
