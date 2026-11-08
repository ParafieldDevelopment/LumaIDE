import "../styles/style.scss";
import {createApp} from "vue";
import Debugging from "~/vue/Debugging.vue";
import { installContextMenu } from "~/vue/composables/contextMenu";

const app = createApp(Debugging)
installContextMenu(app)
app.mount("#app")
