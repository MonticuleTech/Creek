import "bootstrap/dist/css/bootstrap.min.css";
import "bootstrap";
import "./style.css";
import { attachConsole } from '@tauri-apps/plugin-log';

attachConsole();

import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import router from "./router";

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount("#app");
