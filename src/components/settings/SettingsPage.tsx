import { useState, type ReactNode } from "react";
import { toast } from "sonner";
import {
  disableAutostart,
  enableAutostart,
  exportLogs,
  updateAppSettings,
} from "@/lib/tauri";
import type { AppSettings as AppSettingsType } from "@/lib/types";
import { useAppSettings } from "@/hooks";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";

function SettingRow({
  title,
  description,
  control,
}: {
  title: string;
  description: string;
  control: ReactNode;
}) {
  return (
    <div className="grid gap-3 border-b border-slate-200 py-5 sm:grid-cols-[minmax(0,1fr)_220px] sm:items-center">
      <div className="min-w-0">
        <div className="text-sm font-semibold text-slate-950">{title}</div>
        <div className="mt-1 text-xs leading-5 text-slate-500">{description}</div>
      </div>
      <div className="flex min-w-0 justify-start sm:justify-end">{control}</div>
    </div>
  );
}

export function SettingsPage() {
  const {
    settings,
    setSettings,
    autostartEnabled,
    setAutostartEnabled,
    proxyStatus,
    loading,
    error,
  } = useAppSettings();
  const [autostartLoading, setAutostartLoading] = useState(false);

  const proxyRunning = proxyStatus?.running ?? false;

  const applySettings = async (nextSettings: AppSettingsType) => {
    if (settings && proxyRunning && nextSettings.proxy_port !== settings.proxy_port) {
      toast.error("请先停止代理，再修改端口");
      return;
    }

    setSettings(nextSettings);
    try {
      await updateAppSettings(nextSettings);
    } catch (error) {
      toast.error(`保存失败: ${error}`);
    }
  };

  const handleToggleAutostart = async (enabled: boolean) => {
    setAutostartLoading(true);
    try {
      if (enabled) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }
      setAutostartEnabled(enabled);
      toast.success("开机启动已更新");
    } catch (error) {
      toast.error(`启动设置失败: ${error}`);
    } finally {
      setAutostartLoading(false);
    }
  };

  const handleExport = async (format: "json" | "csv") => {
    try {
      await exportLogs({ format });
      toast.success(`日志已导出为 ${format.toUpperCase()}`);
    } catch (error) {
      toast.error(`导出失败: ${error}`);
    }
  };

  if (loading || !settings) {
    return (
      <div className="mx-auto flex max-w-3xl flex-col gap-6 fade-in">
        <div className="border-b border-slate-200 pb-5">
          <h1 className="text-2xl font-semibold tracking-normal text-slate-950">设置</h1>
          <p className="mt-2 text-sm text-slate-500">
            {error ? `加载配置失败: ${error}` : "正在加载当前配置..."}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6 fade-in">
      <div className="border-b border-slate-200 pb-5">
        <h1 className="text-2xl font-semibold tracking-normal text-slate-950">设置</h1>
        <p className="mt-2 text-sm text-slate-500">更改后自动生效。</p>
      </div>

      <div>
        <SettingRow
          title="代理端口"
          description={proxyRunning ? "代理运行中，停止代理后可修改端口。" : "本地代理服务监听的端口。"}
          control={
            <Input
              type="number"
              min={1}
              max={65535}
              value={settings.proxy_port}
              disabled={proxyRunning}
              onChange={(e) => {
                if (proxyRunning) return;
                const proxyPort = e.target.valueAsNumber;
                if (!Number.isFinite(proxyPort) || proxyPort < 1 || proxyPort > 65535) return;
                applySettings({ ...settings, proxy_port: proxyPort });
              }}
              className="w-36"
            />
          }
        />

        <SettingRow
          title="启动时运行代理"
          description="打开应用后自动启动本地代理服务。"
          control={
            <Switch
              checked={settings.auto_start_proxy}
              onChange={(e) =>
                applySettings({ ...settings, auto_start_proxy: e.target.checked })
              }
            />
          }
        />

        <SettingRow
          title="开机启动应用"
          description="系统登录后自动打开此应用。"
          control={
            <Switch
              checked={autostartEnabled}
              onChange={(e) => handleToggleAutostart(e.target.checked)}
              disabled={autostartLoading}
            />
          }
        />

        <SettingRow
          title="日志保留天数"
          description="超过该天数的历史日志会自动清理。"
          control={
            <Input
              type="number"
              min={1}
              max={3650}
              value={settings.log_retention_days}
              onChange={(e) => {
                const logRetentionDays = e.target.valueAsNumber;
                if (
                  !Number.isFinite(logRetentionDays) ||
                  logRetentionDays < 1 ||
                  logRetentionDays > 3650
                ) {
                  return;
                }
                applySettings({
                  ...settings,
                  log_retention_days: logRetentionDays,
                });
              }}
              className="w-36"
            />
          }
        />

        <SettingRow
          title="导出日志"
          description="将当前日志导出为常用数据格式。"
          control={
            <div className="grid w-36 grid-cols-2 gap-2">
              <Button
                onClick={() => handleExport("json")}
                variant="outline"
              >
                JSON
              </Button>
              <Button
                onClick={() => handleExport("csv")}
                variant="outline"
              >
                CSV
              </Button>
            </div>
          }
        />
      </div>
    </div>
  );
}
