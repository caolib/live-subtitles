import { createApp } from "vue";
import Antd from "ant-design-vue";
import "ant-design-vue/dist/reset.css";
import Settings from "./Settings.vue";
import pinia from "./stores";
import "./assets/styles/var.css";

const app = createApp(Settings);
app.use(pinia);
app.use(Antd);
app.mount("#app");
