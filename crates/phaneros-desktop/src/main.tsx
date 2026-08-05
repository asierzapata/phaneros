import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ThemeProvider } from "@/context/ThemeContext";
import { VaultProvider } from "@/context/VaultContext";
import { TelemetryProvider } from "@/context/TelemetryContext";
import { OnboardingProvider } from "@/context/OnboardingContext";
import { ViewProvider } from "@/context/ViewContext";
import { ActivityProvider } from "@/context/ActivityContext";
import { DaemonStatusProvider } from "@/context/DaemonStatusContext";
import "@/styles/tokens.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <DaemonStatusProvider>
        <VaultProvider>
          <TelemetryProvider>
            <ActivityProvider>
              <OnboardingProvider>
                <ViewProvider>
                  <App />
                </ViewProvider>
              </OnboardingProvider>
            </ActivityProvider>
          </TelemetryProvider>
        </VaultProvider>
      </DaemonStatusProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
