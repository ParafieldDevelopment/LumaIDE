import "../styles/style.scss";
import {createApp} from "vue";
import Main from "~/vue/Main.vue";
import { installContextMenu } from "~/vue/composables/contextMenu";

const app = createApp(Main)
installContextMenu(app)
app.mount("#app")
