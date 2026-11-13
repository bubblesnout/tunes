import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// Block the browser context menu / text selection feel; we handle right-click ourselves.
window.addEventListener("contextmenu", (e) => e.preventDefault());

export default mount(App, { target: document.getElementById("app")! });
