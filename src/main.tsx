import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Initialize i18n
import "./i18n";

// Detect and sync dark mode with system preference
function syncDarkMode() {
  const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  document.documentElement.classList.toggle("dark", isDark);
}

// Initial sync
syncDarkMode();

// Listen for system theme changes
window
  .matchMedia("(prefers-color-scheme: dark)")
  .addEventListener("change", syncDarkMode);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
