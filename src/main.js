import { createApp } from "vue";
import { RouterView } from "vue-router";
import router from "./router";
import pinia from "./stores";
import Antd from "ant-design-vue";
import "ant-design-vue/dist/reset.css";
import "./assets/styles/var.css";

// 禁用生产环境的右键上下文菜单
if (import.meta.env.PROD) {
    document.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        return false;
    }, { capture: true });
}

// 创建应用并使用 RouterView 作为根组件
const app = createApp(RouterView);

app.use(pinia);
app.use(router);
app.use(Antd);
app.mount('#app')