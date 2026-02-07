import { createRouter, createWebHistory } from "vue-router";

const routes = [
  {
    path: "/",
    name: "welcome",
    component: () => import("../views/WelcomeView.vue"),
  },
  {
    path: "/workspaces",
    name: "workspaces",
    component: () => import("../views/WorkspaceManagementView.vue"),
  },
  {
    path: "/workspace/:id",
    name: "workspace",
    component: () => import("../views/WorkspaceView.vue"),
  },
  {
    path: "/home",
    name: "home",
    component: () => import("../views/HomeView.vue"),
  },
  {
    path: "/404",
    name: "404",
    component: () => import("../views/NotFound.vue"),
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

export default router;

