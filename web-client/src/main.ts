import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import { useDeviceStore } from "./stores/device";
import "./main.css";

const app = createApp(App);

const pinia = createPinia();
app.use(pinia);
app.use(router);

const deviceStore = useDeviceStore();
deviceStore.init();

app.mount("#app");
