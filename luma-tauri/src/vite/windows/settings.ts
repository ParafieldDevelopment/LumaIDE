import "../styles/style.scss";
import {createApp} from "vue";
import Settings from "~/vue/Settings.vue";
import { installContextMenu } from "~/vue/composables/contextMenu";

const app = createApp(Settings)
installContextMenu(app)
app.mount("#app")
