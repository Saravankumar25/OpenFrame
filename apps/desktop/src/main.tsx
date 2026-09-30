import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { installInvalidation, queryClient } from "./ipc/query";
import { App } from "./app/App";
import "./styles/app.css";

installInvalidation(queryClient);

// Files dropped outside an explicit drop zone must never navigate the webview away.
window.addEventListener("dragover", (e) => e.preventDefault());
window.addEventListener("drop", (e) => e.preventDefault());

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
