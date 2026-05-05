import "@fontsource/space-grotesk/400.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "@mdi/font/css/materialdesignicons.css";
import "vuetify/styles";

import { createApp } from "vue";
import { createPinia } from "pinia";

import App from "./App.vue";
import router from "./router";
import vuetify from "./plugins/vuetify";
import "./styles/app.scss";

createApp(App)
	.use(createPinia())
	.use(router)
	.use(vuetify)
	.mount("#app");
