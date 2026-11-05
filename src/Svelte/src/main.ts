import "./theme.svelte"; // 描く前に data-theme を決める（明滅させない）
import { mount } from "svelte";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app")! });
