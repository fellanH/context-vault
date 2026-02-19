import { createBrowserRouter } from "react-router";
import { RootLayout } from "./components/RootLayout";
import { Dashboard } from "./pages/Dashboard";
import { Knowledge } from "./pages/Knowledge";
import { Entities } from "./pages/Entities";
import { Events } from "./pages/Events";
import { Search } from "./pages/Search";
import { Login } from "./pages/Login";
import { Register } from "./pages/Register";
import { AuthCallback } from "./pages/AuthCallback";
import { ApiKeys } from "./pages/settings/ApiKeys";
import { Billing } from "./pages/settings/Billing";
import { DataManagement } from "./pages/settings/DataManagement";
import { Account } from "./pages/settings/Account";

export const router = createBrowserRouter([
  { path: "/login", Component: Login },
  { path: "/register", Component: Register },
  { path: "/auth/callback", Component: AuthCallback },
  {
    path: "/",
    Component: RootLayout,
    children: [
      { index: true, Component: Dashboard },
      { path: "search", Component: Search },
      { path: "vault/knowledge", Component: Knowledge },
      { path: "vault/entities", Component: Entities },
      { path: "vault/events", Component: Events },
      { path: "settings/api-keys", Component: ApiKeys },
      { path: "settings/billing", Component: Billing },
      { path: "settings/data", Component: DataManagement },
      { path: "settings/account", Component: Account },
    ],
  },
]);
