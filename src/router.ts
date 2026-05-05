import { createRouter, createWebHistory } from "vue-router";

import AppShell from "./components/AppShell.vue";
import DashboardView from "./views/DashboardView.vue";
import SettingsView from "./views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: AppShell,
      children: [
        {
          path: "",
          name: "dashboard",
          component: DashboardView,
        },
        {
          path: "settings",
          name: "settings",
          component: SettingsView,
        },
      ],
    },
  ],
});

export default router;
