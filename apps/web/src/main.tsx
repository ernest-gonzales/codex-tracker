import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

const bootScreen = document.getElementById("boot-screen");
if (bootScreen) {
  window.setTimeout(() => {
    window.requestAnimationFrame(() => {
      bootScreen.classList.add("is-hidden");
      window.setTimeout(() => bootScreen.remove(), 500);
    });
  }, 1000);
}
