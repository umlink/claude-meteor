
import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { EmptyState } from "@/components/shared/EmptyState";
import type { Provider } from "@/lib/types";
import { useProviders } from "@/hooks";
import { ProviderHeader } from "./components/ProviderHeader";
import { ProviderCard } from "./components/ProviderCard";
import { ProviderDialog } from "./components/ProviderDialog";
import { DeleteDialog } from "./components/DeleteDialog";

export function ProviderList() {
  const { providers, create, update, remove } = useProviders();
  const [showForm, setShowForm] = useState(false);
  const [editProvider, setEditProvider] = useState<Provider | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Provider | null>(null);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [form, setForm] = useState({
    name: "",
    base_url: "",
    api_key: "",
    protocol: "anthropic" as "anthropic" | "openai",
    model_mapping: "",
    auth_header: "x-api-key" as "x-api-key" | "bearer",
    keyword: "sonnet",
    enabled: false,
  });

  const openCreate = () => {
    setForm({
      name: "",
      base_url: "",
      api_key: "",
      protocol: "anthropic",
      model_mapping: "",
      auth_header: "x-api-key",
      keyword: "sonnet",
      enabled: providers.length === 0,
    });
    setEditProvider(null);
    setShowForm(true);
  };

  const openEdit = (provider: Provider) => {
    setForm({
      name: provider.name,
      base_url: provider.base_url,
      api_key: "",
      protocol: provider.protocol,
      model_mapping: provider.model_mapping || "",
      auth_header: provider.auth_header,
      keyword: provider.keyword,
      enabled: provider.enabled,
    });
    setEditProvider(provider);
    setShowForm(true);
  };

  const canSaveProvider =
    form.name.trim().length > 0 &&
    form.base_url.trim().length > 0 &&
    form.keyword.trim().length > 0 &&
    (Boolean(editProvider) || form.api_key.trim().length > 0);

  const handleSave = async () => {
    if (!canSaveProvider) {
      toast.error("请填写必填字段");
      return;
    }

    try {
      if (editProvider) {
        await update({
          id: editProvider.id,
          name: form.name,
          base_url: form.base_url,
          api_key: form.api_key || undefined,
          protocol: form.protocol,
          model_mapping: form.model_mapping || undefined,
          auth_header: form.auth_header,
          keyword: form.keyword,
          enabled: form.enabled,
        });
        toast.success("提供商已更新");
      } else {
        await create({
          name: form.name,
          base_url: form.base_url,
          api_key: form.api_key,
          protocol: form.protocol,
          model_mapping: form.model_mapping || undefined,
          auth_header: form.auth_header,
          keyword: form.keyword,
          enabled: form.enabled,
        });
        toast.success("提供商已创建");
      }
      setShowForm(false);
    } catch (error) {
      toast.error(`保存失败: ${error}`);
    }
  };

  const handleDelete = async () => {
    if (!pendingDelete) return;
    const deletingActive = pendingDelete.enabled;
    const hasFallbackProvider = providers.some((provider) => provider.id !== pendingDelete.id);
    try {
      await remove(pendingDelete.id);
      toast.success(
        deletingActive && hasFallbackProvider
          ? "提供商已删除，已自动切换到下一个可用提供商"
          : "提供商已删除"
      );
      setPendingDelete(null);
    } catch (error) {
      toast.error(`删除失败: ${error}`);
    }
  };

  const handleToggle = async (provider: Provider) => {
    if (togglingId) return;
    if (provider.enabled) {
      toast.message("请直接启用其他提供商来切换，系统会始终保留一个启用项");
      return;
    }

    setTogglingId(provider.id);
    try {
      await update({
        id: provider.id,
        name: provider.name,
        base_url: provider.base_url,
        protocol: provider.protocol,
        model_mapping: provider.model_mapping || undefined,
        auth_header: provider.auth_header,
        keyword: provider.keyword,
        enabled: !provider.enabled,
      });
      toast.success("已切换为当前使用的提供商");
    } catch (error) {
      toast.error(`切换失败: ${error}`);
    } finally {
      setTogglingId(null);
    }
  };

  if (providers.length === 0 && !showForm) {
    return (
      <div className="flex flex-col gap-6 fade-in">
        <ProviderHeader providersCount={0} enabledCount={0} onAddClick={openCreate} />
        <EmptyState
          icon={<ShieldCheck className="h-10 w-10" />}
          title="暂无提供商"
          description="添加提供商后，启用其中一个即可开始转发请求"
        />
        <ProviderDialog
          editProvider={editProvider}
          form={form}
          onChange={setForm}
          onClose={() => setShowForm(false)}
          onSave={handleSave}
          open={showForm}
          canSave={canSaveProvider}
        />
      </div>
    );
  }

  const enabledCount = providers.filter((p) => p.enabled).length;

  return (
    <div className="flex flex-col gap-6 fade-in">
      <ProviderHeader providersCount={providers.length} enabledCount={enabledCount} onAddClick={openCreate} />

      <div className="flex flex-col gap-4">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            togglingId={togglingId}
            disableToggle={provider.enabled}
            onToggle={handleToggle}
            onEdit={openEdit}
            onDelete={() => setPendingDelete(provider)}
          />
        ))}
      </div>

      <ProviderDialog
        editProvider={editProvider}
        form={form}
        onChange={setForm}
        onClose={() => setShowForm(false)}
        onSave={handleSave}
        open={showForm}
        canSave={canSaveProvider}
      />
      <DeleteDialog
        onClose={() => setPendingDelete(null)}
        onConfirm={handleDelete}
        provider={pendingDelete}
      />
    </div>
  );
}
