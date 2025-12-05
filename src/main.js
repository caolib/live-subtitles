import { createApp } from "vue";
import App from "./App.vue";
import pinia from "./stores";
import "./assets/styles/var.css";

const app = createApp(App);
app.use(pinia);
app.mount("#app");
