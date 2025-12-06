import { createApp } from "vue";
import { RouterView } from "vue-router";
import router from "./router";
import pinia from "./stores";
import Antd from "ant-design-vue";
import "ant-design-vue/dist/reset.css";
import "./assets/styles/var.css";

// 创建应用并使用 RouterView 作为根组件
const app = createApp(RouterView);

app.use(pinia);
app.use(router);
app.use(Antd);
app.mount('#app')