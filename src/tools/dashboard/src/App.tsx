// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0
import { Dashboard } from "./components/Dashboard";
import { ThemeProvider } from "./components/ThemeProvider";

export default function App() {
  return (
    <ThemeProvider>
      <div className="min-h-screen bg-background">
        <Dashboard />
      </div>
    </ThemeProvider>
  );
}