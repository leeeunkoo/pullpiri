// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

import { useState } from "react";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Switch } from "./ui/switch";
import { RefreshCw, /*Bell, */User, Search, Command, Sun, Moon } from "lucide-react";  // 2025-09-23 comment out
import { Input } from "./ui/input";
import { useTheme } from "./ThemeProvider";
//import { useClusterHealth } from "./ui/use-cluster-health"; // 2025-09-23 comment out

interface Pod {
  name: string;
  image: string;
  labels: Record<string, string>;
  node: string;
  status: string;
  cpuUsage: string;
  memoryUsage: string;
  age: string;
  ready: string;
  restarts: number;
  ip: string;
}

interface HeaderProps {
  compact?: boolean;
  mobile?: boolean;
  podCount?: number;
  pods: Pod[];
}

export function Header({ compact = false, mobile = false, podCount/*, pods */}: HeaderProps) { // 2025-09-23 comment out
  const { theme, toggleTheme } = useTheme();
  //const clusterHealth = useClusterHealth(pods); // 2025-09-23 comment out
  const [demoMode, setDemoMode] = useState(false);
  const [dashboardIP, setDashboardIP] = useState("192.168.1.10");

  const handleDemoToggle = async () => {
    const newDemoMode = !demoMode;
    setDemoMode(newDemoMode);

  };

  if (mobile) {
    return (
      <header className="h-16 bg-card/60 backdrop-blur-xl border-b border-border shadow-lg px-4 flex items-center justify-between relative">
        {/* Background gradient */}
        <div className="absolute inset-0 bg-gradient-to-r from-muted/10 to-muted/20 dark:from-muted/5 dark:to-muted/10"></div>
        
        <div className="flex items-center gap-2 relative z-10">
          <Badge className="gap-2 px-2 py-1 bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 text-xs">
            <div className="w-2 h-2 bg-slate-500 rounded-full"></div>
            {podCount || 0} Pods
          </Badge>
        </div>
        
        <div className="flex items-center gap-2 relative z-10">
          <Button 
            variant="ghost" 
            size="sm" 
            className="w-8 h-8 rounded-lg bg-card/60 hover:bg-card/80"
            onClick={toggleTheme}
          >
            {theme === 'light' ? (
              <Moon className="w-3 h-3" />
            ) : (
              <Sun className="w-3 h-3" />
            )}
          </Button>
          <Button variant="ghost" size="sm" className="w-8 h-8 rounded-lg bg-primary hover:bg-primary/80">
            <User className="w-3 h-3 text-primary-foreground" />
          </Button>
        </div>
      </header>
    );
  }

  const headerHeight = mobile ? "h-16" : compact ? "h-16" : "h-20";
  const paddingX = mobile ? "px-4" : compact ? "px-6" : "px-8";

  return (
    <header className={`${headerHeight} bg-card/60 backdrop-blur-xl border-b border-border shadow-lg ${paddingX} flex items-center justify-between relative`}>
      {/* Background gradient */}
      <div className="absolute inset-0 bg-gradient-to-r from-muted/10 to-muted/20 dark:from-muted/5 dark:to-muted/10"></div>
      
      <div className="flex items-center gap-3 lg:gap-6 relative z-10 flex-1 min-w-0">

        
        {!compact && (
          <div className="flex items-center gap-2 lg:gap-3 min-w-0">

            <Badge className="gap-2 px-2 lg:px-3 py-1 lg:py-1.5 bg-orange-500 dark:bg-orange-600 text-white border-orange-600 dark:border-orange-700 hover:bg-orange-600 dark:hover:bg-orange-700 whitespace-nowrap text-xs lg:text-sm hidden sm:flex">
              <div className="w-2 lg:w-2.5 h-2 lg:h-2.5 bg-white rounded-full"></div>
              {podCount || 0} Pods Running
            </Badge>
          </div>
        )}



        {/* Search Bar - Only on larger screens */}
        {!compact && (
          <div className="relative ml-2 lg:ml-4 hidden xl:block flex-1 max-w-sm">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              placeholder="Search resources..."
              className="w-full pl-10 pr-12 h-8 lg:h-10 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all text-sm"
            />
            <div className="absolute right-3 top-1/2 transform -translate-y-1/2 flex items-center gap-1">
              <Command className="w-3 h-3 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">K</span>
            </div>
          </div>
        )}
      </div>
      
      <div className="flex items-center gap-2 lg:gap-4 relative z-10">
        {/* IP Configuration and Demo Toggle */}
        {!compact && (
          <div className="flex items-center gap-3 hidden lg:flex">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-foreground whitespace-nowrap">IP Input</span>
              <div className="relative">
                <Input
                  placeholder="Dashboard IP..."
                  value={dashboardIP}
                  onChange={(e) => setDashboardIP(e.target.value)}
                  className="w-36 h-9 bg-background border-2 border-primary/20 shadow-lg hover:shadow-xl focus:border-primary focus:shadow-xl transition-all text-sm font-mono font-medium text-[rgba(141,138,138,1)]"
                />
              </div>
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm font-medium text-foreground">Demo</span>
              <Switch
                checked={demoMode}
                onCheckedChange={handleDemoToggle}
                className="data-[state=checked]:bg-emerald-500 data-[state=unchecked]:bg-muted scale-125"
              />
            </div>
          </div>
        )}
        
        <Button 
          variant="ghost" 
          size="sm" 
          className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all`}
          onClick={toggleTheme}
        >
          {theme === 'light' ? (
            <Moon className="w-4 h-4" />
          ) : (
            <Sun className="w-4 h-4" />
          )}
        </Button>
        <Button variant="ghost" size="sm" className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all hidden sm:flex`}>
          <RefreshCw className="w-4 h-4" />
        </Button>

        <Button variant="ghost" size="sm" className={`${compact ? 'w-8 h-8' : 'w-10 h-10'} rounded-xl bg-card/60 hover:bg-card/80 hover:shadow-md transition-all`}>
          <User className="w-4 h-4 text-primary-foreground" />
        </Button>
      </div>
    </header>
  );
}