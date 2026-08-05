import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ThemeProvider } from "@/context/ThemeContext";
import { VaultProvider } from "@/context/VaultContext";
import { TelemetryProvider } from "@/context/TelemetryContext";
import { OnboardingProvider } from "@/context/OnboardingContext";
import { ViewProvider } from "@/context/ViewContext";
import "@/styles/tokens.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <VaultProvider>
        <TelemetryProvider>
          <OnboardingProvider>
            <ViewProvider>
              <App />
            </ViewProvider>
          </OnboardingProvider>
        </TelemetryProvider>
      </VaultProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
