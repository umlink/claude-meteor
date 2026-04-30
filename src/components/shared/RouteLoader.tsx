import { LoaderCircle } from "lucide-react";

export function RouteLoader() {
  return (
    <div className="flex min-h-[320px] items-center justify-center">
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <LoaderCircle className="h-4 w-4 animate-spin" />
        <span>正在加载页面...</span>
      </div>
    </div>
  );
}
