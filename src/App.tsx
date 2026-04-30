import { Suspense, lazy } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { Toaster } from "sonner";
import { TopNav } from "./components/layout/TopNav";
import { RouteLoader } from "./components/shared/RouteLoader";

const DashboardPage = lazy(() =>
  import("./components/dashboard/DashboardPage").then((module) => ({
    default: module.DashboardPage,
  }))
);
const ProviderList = lazy(() =>
  import("./components/providers/ProviderList").then((module) => ({
    default: module.ProviderList,
  }))
);
const RequestLog = lazy(() =>
  import("./components/logs/RequestLog").then((module) => ({
    default: module.RequestLog,
  }))
);
const SettingsPage = lazy(() =>
  import("./components/settings/SettingsPage").then((module) => ({
    default: module.SettingsPage,
  }))
);

function AppLayout() {
  return (
    <div className="min-h-screen bg-background lg:pl-[72px]">
      <TopNav />
      <main className="mx-auto max-w-7xl px-6 py-8 pb-24 lg:pb-8">
        <Suspense fallback={<RouteLoader />}>
          <Routes>
            <Route path="/" element={<Navigate to="/dashboard" replace />} />
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/providers" element={<ProviderList />} />
            <Route path="/logs" element={<RequestLog />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </Suspense>
      </main>
    </div>
  );
}

export default function App() {
  return (
    <HashRouter>
      <AppLayout />
      <Toaster
        richColors
        position="top-right"
        toastOptions={{
          style: {
            background: 'hsl(var(--card))',
            border: '1px solid hsl(var(--border))',
            boxShadow: '0 10px 15px -3px rgb(0 0 0 / 0.1)'
          },
        }}
      />
    </HashRouter>
  );
}
