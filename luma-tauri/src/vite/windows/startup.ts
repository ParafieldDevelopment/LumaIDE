import "../styles/style.scss";
import {createApp} from "vue";
import Startup from "~/vue/Startup.vue";
import { installContextMenu } from "~/vue/composables/contextMenu";

const app = createApp(Startup)
installContextMenu(app)
app.mount("#app")
