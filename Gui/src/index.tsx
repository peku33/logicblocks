import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "setimmediate";
import App from "./App";

const container = document.getElementById("root");
const root = createRoot(container!);
root.render(
  <StrictMode>
    <App />
  </StrictMode>,
);
