import { createApp } from "vue";
import Settings from "./Settings.vue";
import pinia from "./stores";
import "./assets/styles/var.css";

const app = createApp(Settings);
app.use(pinia);
app.mount("#app");
