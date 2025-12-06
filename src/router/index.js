import { createRouter, createWebHistory } from "vue-router";
import App from "../App.vue";
import Settings from "../Settings.vue";

const router = createRouter({
    history: createWebHistory(),
    routes: [
        {
            path: "/",
            name: "home",
            component: App
        },
        {
            path: "/settings",
            name: "settings",
            component: Settings
        }
    ]
});

export default router;
