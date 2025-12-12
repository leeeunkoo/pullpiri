// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0
import React, { useState, useRef } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Input } from "./ui/input";
import {
  GitBranch,
  Box,
  // ArrowRight,  // 2025-09-23 comment out
  Package,
  Activity,
  Cpu,
  MemoryStick,
  Clock,
  Play,
  Pause,
  Settings,
  Share2,
  ZoomIn,
  ZoomOut,
  RotateCcw,
  Search,
} from "lucide-react";

interface Pod {
  name: string;
  image: string;
  status: string;
  cpuUsage: string;
  memoryUsage: string;
  node: string;
}

interface Package {
  name: string;
  pods: Pod[];
}

interface Scenario {
  id: string;
  name: string;
  description: string;
  status: "running" | "stopped" | "starting";
  package: Package;
  lastRun: string;
}

interface ScenariosProps {
  namespace: string;
}

interface Node {
  id: string;
  label: string;
  type: "scenario" | "pod";
  status: "running" | "stopped" | "starting" | "pending";
  x: number;
  y: number;
  color: string;
}

interface Edge {
  from: string;
  to: string;
}

export function Scenarios({ namespace }: ScenariosProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState(1);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [connections, setConnections] = useState<Edge[]>([]);

  // Mock scenario data with state management
  const [scenarios, setScenarios] = useState<Scenario[]>([
    {
      id: "ad_driving",
      name: "AD Driving",
      description:
        "Autonomous driving scenario with full self-driving capabilities",
      status: "running",
      lastRun: "2 minutes ago",
      package: {
        name: "ad_driving_package",
        pods: [
          {
            name: "자동주행알고리즘",
            image: "ai/autonomous-driving:v2.1",
            status: "Running",
            cpuUsage: "850m",
            memoryUsage: "2.1Gi",
            node: "gpu-node-1",
          },
          {
            name: "차량상태모니터링",
            image: "monitoring/vehicle-state:v1.5",
            status: "Running",
            cpuUsage: "120m",
            memoryUsage: "256Mi",
            node: "worker-node-2",
          },
        ],
      },
    },
    {
      id: "manual_driving",
      name: "Manual Driving",
      description: "Manual driving scenario with driver assistance systems",
      status: "stopped",
      lastRun: "1 hour ago",
      package: {
        name: "ma_driving_package",
        pods: [
          {
            name: "운전자보조시스템",
            image: "adas/driver-assist:v1.8",
            status: "Stopped",
            cpuUsage: "0m",
            memoryUsage: "0Mi",
            node: "worker-node-1",
          },
          {
            name: "차량상태모니터링",
            image: "monitoring/vehicle-state:v1.5",
            status: "Running",
            cpuUsage: "115m",
            memoryUsage: "240Mi",
            node: "worker-node-2",
          },
        ],
      },
    },
    {
      id: "emergency_mode",
      name: "Emergency Mode",
      description: "Emergency response scenario for critical situations",
      status: "starting",
      lastRun: "Never",
      package: {
        name: "emergency_package",
        pods: [
          {
            name: "응급제동시스템",
            image: "safety/emergency-brake:v2.0",
            status: "Starting",
            cpuUsage: "200m",
            memoryUsage: "512Mi",
            node: "safety-node-1",
          },
          {
            name: "비상통신모듈",
            image: "comm/emergency-comm:v1.2",
            status: "Pending",
            cpuUsage: "0m",
            memoryUsage: "0Mi",
            node: "comm-node-1",
          },
          {
            name: "차량상태모니터링",
            image: "monitoring/vehicle-state:v1.5",
            status: "Running",
            cpuUsage: "140m",
            memoryUsage: "280Mi",
            node: "worker-node-2",
          },
        ],
      },
    },
  ]);

  // Dependencies data for the graph view
  const nodes: Node[] = [
    // Scenarios
    {
      id: "ad_driving",
      label: "AD Driving",
      type: "scenario",
      status: "running",
      x: 200,
      y: 100,
      color: "#3b82f6",
    },
    {
      id: "manual_driving",
      label: "Manual Driving",
      type: "scenario",
      status: "stopped",
      x: 200,
      y: 300,
      color: "#3b82f6",
    },
    {
      id: "emergency_mode",
      label: "Emergency Mode",
      type: "scenario",
      status: "starting",
      x: 200,
      y: 500,
      color: "#3b82f6",
    },
    // Pods
    {
      id: "autonomous_algo",
      label: "자동주행알고리즘",
      type: "pod",
      status: "running",
      x: 500,
      y: 80,
      color: "#10b981",
    },
    {
      id: "vehicle_monitor",
      label: "차량상태모니터링",
      type: "pod",
      status: "running",
      x: 600,
      y: 250,
      color: "#10b981",
    },
    {
      id: "driver_assist",
      label: "운전자보조시스템",
      type: "pod",
      status: "stopped",
      x: 500,
      y: 320,
      color: "#10b981",
    },
    {
      id: "emergency_brake",
      label: "응급제동시스템",
      type: "pod",
      status: "starting",
      x: 500,
      y: 480,
      color: "#10b981",
    },
    {
      id: "emergency_comm",
      label: "비상통신모듈",
      type: "pod",
      status: "pending",
      x: 500,
      y: 520,
      color: "#10b981",
    },
  ];

  const edges: Edge[] = [
    // AD Driving connections
    { from: "ad_driving", to: "autonomous_algo" },
    { from: "ad_driving", to: "vehicle_monitor" },

    // Manual Driving connections
    { from: "manual_driving", to: "driver_assist" },
    { from: "manual_driving", to: "vehicle_monitor" },

    // Emergency Mode connections
    { from: "emergency_mode", to: "emergency_brake" },
    { from: "emergency_mode", to: "emergency_comm" },
    { from: "emergency_mode", to: "vehicle_monitor" },
  ];

  // Initialize connections state
  React.useEffect(() => {
    setConnections(edges);
  }, []);

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case "running":
        return "bg-emerald-500/10 text-emerald-600 border-emerald-200 dark:border-emerald-800";
      case "stopped":
        return "bg-gray-500/10 text-gray-600 border-gray-200 dark:border-gray-700";
      case "starting":
        return "bg-blue-500/10 text-blue-600 border-blue-200 dark:border-blue-800";
      case "pending":
        return "bg-amber-500/10 text-amber-600 border-amber-200 dark:border-amber-800";
      default:
        return "bg-gray-500/10 text-gray-600 border-gray-200 dark:border-gray-700";
    }
  };

  const getScenarioStatusIcon = (status: string) => {
    switch (status) {
      case "running":
        return <Play className="w-4 h-4" />;
      case "stopped":
        return <Pause className="w-4 h-4" />;
      case "starting":
        return <Activity className="w-4 h-4 animate-pulse" />;
      default:
        return <Settings className="w-4 h-4" />;
    }
  };

  // Dependencies helper functions
  const filteredNodes = nodes.filter((node) =>
    node.label.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredEdges = connections.filter((edge) => {
    const fromNode = filteredNodes.find((n) => n.id === edge.from);
    const toNode = filteredNodes.find((n) => n.id === edge.to);
    return fromNode && toNode;
  });

  /*const getNodeColor = (node: Node) => { // 2025-09-23 comment out
    if (selectedNode === node.id) {
      return "#dc2626"; // Red for selected
    }
    return node.color;
  }; */

  const handleZoomIn = () => {
    setZoom((prev) => Math.min(prev * 1.2, 3));
  };

  const handleZoomOut = () => {
    setZoom((prev) => Math.max(prev / 1.2, 0.3));
  };

  const handleReset = () => {
    setZoom(1);
    setPanX(0);
    setPanY(0);
    setSelectedNode(null);
  };

  const handleNodeClick = (nodeId: string) => {
    setSelectedNode(selectedNode === nodeId ? null : nodeId);
  };

  // Calculate ONLY directly connected nodes for highlighting - Corrected logic
  const getConnectedNodes = (nodeId: string) => {
    const connected = new Set<string>();

    console.log(`🔍 Finding connections for node: ${nodeId}`);

    // Only add nodes that have DIRECT connections to the selected node
    connections.forEach((edge) => {
      if (edge.from === nodeId) {
        connected.add(edge.to);
        console.log(`  ➡️ Found outgoing connection: ${nodeId} → ${edge.to}`);
      } else if (edge.to === nodeId) {
        connected.add(edge.from);
        console.log(`  ⬅️ Found incoming connection: ${edge.from} → ${nodeId}`);
      }
    });

    console.log(`  📊 Connected nodes for ${nodeId}:`, Array.from(connected));
    return connected;
  };

  const connectedNodes = selectedNode
    ? getConnectedNodes(selectedNode)
    : new Set<string>();

  // Map node IDs to pod names
  const getNodeIdToPodNameMap = () => {
    const map: Record<string, string> = {
      autonomous_algo: "자동주행알고리즘",
      vehicle_monitor: "차량상태모니터링",
      driver_assist: "운전자보조시스템",
      emergency_brake: "응급제동시스템",
      emergency_comm: "비상통신모듈",
    };
    return map;
  };

  // Handle edge removal
  const handleRemoveEdge = (fromId: string, toId: string) => {
    console.log("🔥 Removing edge:", fromId, "->", toId);

    // Update connections state
    setConnections((prev) => {
      const newConnections = prev.filter(
        (edge) => !(edge.from === fromId && edge.to === toId)
      );
      console.log("📊 Updated connections:", newConnections.length);
      return newConnections;
    });

    // Get the actual pod name from node ID
    const nodeIdToPodName = getNodeIdToPodNameMap();
    const podName = nodeIdToPodName[toId] || toId;

    // Update scenarios state - remove pod from scenario package
    setScenarios((prev) => {
      const updatedScenarios = prev.map((scenario) => {
        // If this scenario is the source (fromId), remove the target pod (toId)
        if (scenario.id === fromId) {
          const updatedPods = scenario.package.pods.filter(
            (pod) => pod.name !== podName
          );
          console.log(
            `🎯 Scenario ${fromId}: removed pod ${podName} (node: ${toId}), ${scenario.package.pods.length} -> ${updatedPods.length} pods`
          );

          return {
            ...scenario,
            package: {
              ...scenario.package,
              pods: updatedPods,
            },
          };
        }
        return scenario;
      });

      console.log(
        "📋 Updated scenarios:",
        updatedScenarios.map((s) => `${s.name}: ${s.package.pods.length} pods`)
      );
      return updatedScenarios;
    });
  };

  // Reset connections and restore original scenario data
  const handleResetConnections = () => {
    setConnections(edges);

    // Restore original scenario data
    setScenarios([
      {
        id: "ad_driving",
        name: "AD Driving",
        description:
          "Autonomous driving scenario with full self-driving capabilities",
        status: "running",
        lastRun: "2 minutes ago",
        package: {
          name: "ad_driving_package",
          pods: [
            {
              name: "자동주행알고리즘",
              image: "ai/autonomous-driving:v2.1",
              status: "Running",
              cpuUsage: "850m",
              memoryUsage: "2.1Gi",
              node: "gpu-node-1",
            },
            {
              name: "차량상태모니터링",
              image: "monitoring/vehicle-state:v1.5",
              status: "Running",
              cpuUsage: "120m",
              memoryUsage: "256Mi",
              node: "worker-node-2",
            },
          ],
        },
      },
      {
        id: "manual_driving",
        name: "Manual Driving",
        description: "Manual driving scenario with driver assistance systems",
        status: "stopped",
        lastRun: "1 hour ago",
        package: {
          name: "ma_driving_package",
          pods: [
            {
              name: "운전자보조시스템",
              image: "adas/driver-assist:v1.8",
              status: "Stopped",
              cpuUsage: "0m",
              memoryUsage: "0Mi",
              node: "worker-node-1",
            },
            {
              name: "차량상태모니터링",
              image: "monitoring/vehicle-state:v1.5",
              status: "Stopped",
              cpuUsage: "0m",
              memoryUsage: "0Mi",
              node: "worker-node-2",
            },
          ],
        },
      },
      {
        id: "emergency_mode",
        name: "Emergency Mode",
        description: "Emergency response scenario for critical situations",
        status: "starting",
        lastRun: "Never",
        package: {
          name: "emergency_package",
          pods: [
            {
              name: "응급제동시스템",
              image: "safety/emergency-brake:v2.0",
              status: "Starting",
              cpuUsage: "200m",
              memoryUsage: "512Mi",
              node: "safety-node-1",
            },
            {
              name: "비상통신모듈",
              image: "comm/emergency-comm:v1.2",
              status: "Pending",
              cpuUsage: "0m",
              memoryUsage: "0Mi",
              node: "comm-node-1",
            },
            {
              name: "차량상태모니터링",
              image: "monitoring/vehicle-state:v1.5",
              status: "Running",
              cpuUsage: "140m",
              memoryUsage: "280Mi",
              node: "worker-node-2",
            },
          ],
        },
      },
    ]);
  };

  // Get edge ID for hovering
  const getEdgeId = (edge: Edge) => `${edge.from}-${edge.to}`;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground text-[20px]">
              PULLPIRI Scenarios
            </h1>
          </div>
          <p className="text-muted-foreground">
            Manage and monitor scenario-based pod deployments and dependencies
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="secondary" className="gap-1">
            <Package className="w-3 h-3" />
            Namespace: {namespace}
          </Badge>
        </div>
      </div>

      {/* Enhanced Tabs for Scenarios and Dependencies */}
      <Card className="bg-card/30 backdrop-blur-sm border-border/50 shadow-lg">
        <CardContent className="p-1">
          <Tabs defaultValue="scenarios" className="w-full">
            <TabsList className="grid w-full grid-cols-2 h-14 p-1 bg-secondary rounded-lg border border-border shadow-sm">
              <TabsTrigger
                value="scenarios"
                className="flex items-center justify-center gap-3 h-12 px-6 text-base font-medium data-[state=active]:bg-primary data-[state=active]:text-primary-foreground data-[state=inactive]:bg-background data-[state=inactive]:text-muted-foreground data-[state=active]:shadow-lg transition-all duration-200 rounded-md"
              >
                <GitBranch className="w-5 h-5" />
                Scenarios
              </TabsTrigger>
              <TabsTrigger
                value="dependencies"
                className="flex items-center justify-center gap-3 h-12 px-6 text-base font-medium data-[state=active]:bg-primary data-[state=active]:text-primary-foreground data-[state=inactive]:bg-background data-[state=inactive]:text-muted-foreground data-[state=active]:shadow-lg transition-all duration-200 rounded-md"
              >
                <Share2 className="w-5 h-5" />
                Dependencies
              </TabsTrigger>
            </TabsList>

            {/* Scenarios Tab */}
            <TabsContent value="scenarios" className="space-y-6 mt-6">
              {/* Scenarios Grid */}
              <div className="grid grid-cols-1 xl:grid-cols-2 2xl:grid-cols-3 gap-6">
                {scenarios.map((scenario) => (
                  <Card
                    key={scenario.id}
                    className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg"
                  >
                    <CardHeader className="pb-4">
                      <div className="flex items-start justify-between">
                        <div className="space-y-2">
                          <CardTitle className="flex items-center gap-2">
                            <GitBranch className="w-5 h-5 text-primary" />
                            {scenario.name}
                          </CardTitle>
                          <p className="text-sm text-muted-foreground">
                            {scenario.description}
                          </p>
                        </div>
                        <Badge
                          variant="outline"
                          className={`gap-1 ${getStatusColor(scenario.status)}`}
                        >
                          {getScenarioStatusIcon(scenario.status)}
                          {scenario.status.charAt(0).toUpperCase() +
                            scenario.status.slice(1)}
                        </Badge>
                      </div>

                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <span className="flex items-center gap-1">
                          <Clock className="w-3 h-3" />
                          Last run: {scenario.lastRun}
                        </span>
                      </div>
                    </CardHeader>

                    <CardContent className="space-y-4">
                      {/* Package Section */}
                      <div className="p-3 bg-muted/30 rounded-lg border border-border/30">
                        <div className="flex items-center gap-2 mb-3">
                          <Package className="w-4 h-4 text-primary" />
                          <span className="font-medium text-sm">
                            {scenario.package.name}
                          </span>
                        </div>

                        {/* Pods Flow */}
                        <div className="space-y-3">
                          {scenario.package.pods.map((pod, index) => (
                            <div
                              key={`${scenario.id}-${pod.name}`}
                              className="relative"
                            >
                              {/* Connection Line */}
                              {index > 0 && (
                                <div className="absolute -top-6 left-4 w-0.5 h-6 bg-border/50"></div>
                              )}

                              {/* Pod Card */}
                              <div className="bg-card/70 backdrop-blur-sm rounded-lg p-3 border border-border/40 shadow-sm">
                                <div className="flex items-start justify-between mb-2">
                                  <div className="flex items-center gap-2">
                                    <Box className="w-4 h-4 text-chart-1" />
                                    <span className="font-medium text-sm">
                                      {pod.name}
                                    </span>
                                  </div>
                                  <Badge
                                    variant="outline"
                                    className={`text-xs ${getStatusColor(
                                      pod.status
                                    )}`}
                                  >
                                    {pod.status}
                                  </Badge>
                                </div>

                                <div className="space-y-1 text-xs text-muted-foreground">
                                  <div className="flex items-center gap-2">
                                    <span className="font-mono text-xs">
                                      {pod.image}
                                    </span>
                                  </div>

                                  <div className="flex items-center gap-4">
                                    <span className="flex items-center gap-1">
                                      <Cpu className="w-3 h-3" />
                                      {pod.cpuUsage}
                                    </span>
                                    <span className="flex items-center gap-1">
                                      <MemoryStick className="w-3 h-3" />
                                      {pod.memoryUsage}
                                    </span>
                                  </div>

                                  <div className="text-xs">
                                    Node: {pod.node}
                                  </div>
                                </div>
                              </div>
                            </div>
                          ))}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>

              {/* Shared Pods Analysis */}
              <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <GitBranch className="w-5 h-5 text-primary" />
                    Pod Sharing Analysis
                  </CardTitle>
                  <p className="text-sm text-muted-foreground">
                    Analysis of pods shared across multiple scenarios
                  </p>
                </CardHeader>
                <CardContent>
                  <div className="space-y-4">
                    {/* Shared Pod: 차량상태모니터링 */}
                    <div className="p-4 bg-muted/20 rounded-lg border border-border/30">
                      <div className="flex items-center justify-between mb-3">
                        <div className="flex items-center gap-2">
                          <Box className="w-4 h-4 text-chart-2" />
                          <span className="font-medium">차량상태모니터링</span>
                          <Badge variant="secondary" className="text-xs">
                            Shared Pod
                          </Badge>
                        </div>
                        <Badge
                          variant="outline"
                          className={getStatusColor("running")}
                        >
                          <Activity className="w-3 h-3 mr-1" />
                          Running
                        </Badge>
                      </div>

                      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                        <div className="text-sm">
                          <span className="text-muted-foreground">
                            Used in scenarios:
                          </span>
                          <div className="mt-1 space-y-1">
                            <Badge variant="outline" className="text-xs">
                              AD Driving
                            </Badge>
                            <Badge variant="outline" className="text-xs">
                              Manual Driving
                            </Badge>
                            <Badge variant="outline" className="text-xs">
                              Emergency Mode
                            </Badge>
                          </div>
                        </div>

                        <div className="text-sm">
                          <span className="text-muted-foreground">
                            Resource Usage:
                          </span>
                          <div className="mt-1 space-y-1 text-xs">
                            <div>CPU: 115-140m (variable)</div>
                            <div>Memory: 240-280Mi (variable)</div>
                          </div>
                        </div>

                        <div className="text-sm">
                          <span className="text-muted-foreground">
                            Deployment:
                          </span>
                          <div className="mt-1 space-y-1 text-xs">
                            <div>Node: worker-node-2</div>
                            <div>Image: monitoring/vehicle-state:v1.5</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>

            {/* Dependencies Tab */}
            <TabsContent value="dependencies" className="space-y-6 mt-6">
              {/* Controls */}
              <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
                <CardContent className="p-4">
                  <div className="flex items-center justify-between gap-4">
                    <div className="flex items-center gap-2 flex-1">
                      <Search className="w-4 h-4 text-muted-foreground" />
                      <Input
                        placeholder="Search nodes..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="max-w-xs"
                      />
                    </div>

                    <div className="flex items-center gap-1">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleZoomIn}
                      >
                        <ZoomIn className="w-4 h-4" />
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleZoomOut}
                      >
                        <ZoomOut className="w-4 h-4" />
                      </Button>
                      <Button variant="outline" size="sm" onClick={handleReset}>
                        <RotateCcw className="w-4 h-4" />
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleResetConnections}
                      >
                        Reset Connections
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* Dependency Graph */}
              <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Share2 className="w-5 h-5 text-primary" />
                    Dependency Graph
                  </CardTitle>
                  <p className="text-sm text-muted-foreground">
                    Visual representation of scenario and pod dependencies
                  </p>
                </CardHeader>
                <CardContent>
                  <div className="relative w-full h-[700px] bg-gradient-to-br from-muted/10 to-muted/30 rounded-lg border border-border/30 overflow-hidden">
                    <svg
                      ref={svgRef}
                      width="100%"
                      height="100%"
                      className="cursor-grab active:cursor-grabbing"
                      style={{
                        transform: `translate(${panX}px, ${panY}px) scale(${zoom})`,
                      }}
                    >
                      {/* Enhanced Grid Pattern */}
                      <defs>
                        <pattern
                          id="grid"
                          width="30"
                          height="30"
                          patternUnits="userSpaceOnUse"
                        >
                          <path
                            d="M 30 0 L 0 0 0 30"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="0.5"
                            className="text-border/10"
                          />
                        </pattern>

                        {/* Node Gradients */}
                        <linearGradient
                          id="scenarioGradient"
                          x1="0%"
                          y1="0%"
                          x2="100%"
                          y2="100%"
                        >
                          <stop offset="0%" stopColor="#60a5fa" />
                          <stop offset="100%" stopColor="#3b82f6" />
                        </linearGradient>

                        <linearGradient
                          id="podGradient"
                          x1="0%"
                          y1="0%"
                          x2="100%"
                          y2="100%"
                        >
                          <stop offset="0%" stopColor="#34d399" />
                          <stop offset="100%" stopColor="#10b981" />
                        </linearGradient>

                        {/* Shadow Filter */}
                        <filter
                          id="shadow"
                          x="-50%"
                          y="-50%"
                          width="200%"
                          height="200%"
                        >
                          <feDropShadow
                            dx="2"
                            dy="4"
                            stdDeviation="4"
                            floodOpacity="0.2"
                          />
                        </filter>

                        {/* Arrow marker */}
                        <marker
                          id="arrowhead"
                          markerWidth="12"
                          markerHeight="8"
                          refX="10"
                          refY="4"
                          orient="auto"
                        >
                          <polygon points="0 0, 12 4, 0 8" fill="#6b7280" />
                        </marker>

                        {/* Highlighted arrow marker */}
                        <marker
                          id="arrowhead-highlight"
                          markerWidth="12"
                          markerHeight="8"
                          refX="10"
                          refY="4"
                          orient="auto"
                        >
                          <polygon points="0 0, 12 4, 0 8" fill="#dc2626" />
                        </marker>
                      </defs>

                      <rect width="100%" height="100%" fill="url(#grid)" />

                      {/* Render edges with enhanced interactivity */}
                      {filteredEdges.map((edge) => {
                        const fromNode = filteredNodes.find(
                          (n) => n.id === edge.from
                        );
                        const toNode = filteredNodes.find(
                          (n) => n.id === edge.to
                        );

                        if (!fromNode || !toNode) return null;

                        const edgeId = getEdgeId(edge);
                        // Only highlight edges that are directly connected to the selected node
                        const isHighlighted =
                          selectedNode &&
                          ((selectedNode === edge.from &&
                            connectedNodes.has(edge.to)) ||
                            (selectedNode === edge.to &&
                              connectedNodes.has(edge.from)));
                        const isHovered = hoveredEdge === edgeId;

                        // Calculate arrow position
                        const dx = toNode.x - fromNode.x;
                        const dy = toNode.y - fromNode.y;
                        const distance = Math.sqrt(dx * dx + dy * dy);
                        const nodeRadius = toNode.type === "scenario" ? 25 : 20;
                        const arrowX = toNode.x - (dx / distance) * nodeRadius;
                        const arrowY = toNode.y - (dy / distance) * nodeRadius;

                        return (
                          <g key={edgeId}>
                            {/* Main edge line */}
                            <line
                              x1={fromNode.x}
                              y1={fromNode.y}
                              x2={arrowX}
                              y2={arrowY}
                              stroke={
                                isHighlighted || isHovered
                                  ? "#dc2626"
                                  : "#6b7280"
                              }
                              strokeWidth={isHighlighted || isHovered ? 3 : 2}
                              markerEnd={
                                isHighlighted || isHovered
                                  ? "url(#arrowhead-highlight)"
                                  : "url(#arrowhead)"
                              }
                              className="transition-all duration-200 cursor-pointer"
                              onMouseEnter={() => setHoveredEdge(edgeId)}
                              onMouseLeave={() => setHoveredEdge(null)}
                              onClick={() =>
                                handleRemoveEdge(edge.from, edge.to)
                              }
                            />

                            {/* Invisible wider line for easier clicking */}
                            <line
                              x1={fromNode.x}
                              y1={fromNode.y}
                              x2={arrowX}
                              y2={arrowY}
                              stroke="transparent"
                              strokeWidth="12"
                              className="cursor-pointer"
                              onMouseEnter={() => setHoveredEdge(edgeId)}
                              onMouseLeave={() => setHoveredEdge(null)}
                              onClick={() =>
                                handleRemoveEdge(edge.from, edge.to)
                              }
                            />

                            {/* Delete indicator */}
                            {isHovered && (
                              <g>
                                <circle
                                  cx={(fromNode.x + toNode.x) / 2}
                                  cy={(fromNode.y + toNode.y) / 2}
                                  r="8"
                                  fill="#dc2626"
                                  className="animate-pulse cursor-pointer"
                                  onClick={() =>
                                    handleRemoveEdge(edge.from, edge.to)
                                  }
                                />
                                <text
                                  x={(fromNode.x + toNode.x) / 2}
                                  y={(fromNode.y + toNode.y) / 2}
                                  textAnchor="middle"
                                  dy="0.3em"
                                  fontSize="10"
                                  fill="white"
                                  className="cursor-pointer font-medium"
                                  onClick={() =>
                                    handleRemoveEdge(edge.from, edge.to)
                                  }
                                >
                                  ×
                                </text>
                              </g>
                            )}
                          </g>
                        );
                      })}

                      {/* Render nodes with enhanced styling */}
                      {filteredNodes.map((node) => {
                        const isSelected = selectedNode === node.id;
                        const isConnected =
                          selectedNode &&
                          selectedNode !== node.id &&
                          connectedNodes.has(node.id);
                        const shouldHighlight = isSelected || isConnected;

                        // Debug logging
                        if (selectedNode) {
                          console.log(
                            `Node ${node.id}: selected=${isSelected}, connected=${isConnected}, shouldHighlight=${shouldHighlight}`
                          );
                        }

                        return (
                          <g key={node.id}>
                            {/* Node glow effect */}
                            {shouldHighlight && (
                              <circle
                                cx={node.x}
                                cy={node.y}
                                r={node.type === "scenario" ? 35 : 30}
                                fill={
                                  node.type === "scenario"
                                    ? "#3b82f6"
                                    : "#10b981"
                                }
                                fillOpacity="0.2"
                                className="animate-pulse"
                              />
                            )}

                            {/* Main node circle */}
                            <circle
                              cx={node.x}
                              cy={node.y}
                              r={node.type === "scenario" ? 25 : 20}
                              fill={
                                node.type === "scenario"
                                  ? "url(#scenarioGradient)"
                                  : "url(#podGradient)"
                              }
                              stroke={shouldHighlight ? "#dc2626" : "#ffffff"}
                              strokeWidth={shouldHighlight ? 3 : 2}
                              filter="url(#shadow)"
                              className="cursor-pointer transition-all duration-200 hover:opacity-80"
                              onClick={() => handleNodeClick(node.id)}
                            />

                            {/* Node icon */}
                            <text
                              x={node.x}
                              y={node.y}
                              textAnchor="middle"
                              dy="0.3em"
                              fontSize="12"
                              fill="white"
                              className="cursor-pointer font-medium pointer-events-none"
                            >
                              {node.type === "scenario" ? "S" : "P"}
                            </text>

                            {/* Node label */}
                            <text
                              x={node.x}
                              y={node.y + (node.type === "scenario" ? 40 : 35)}
                              textAnchor="middle"
                              className="text-xs fill-current text-foreground font-medium cursor-pointer"
                              onClick={() => handleNodeClick(node.id)}
                            >
                              {node.label}
                            </text>

                            {/* Status indicator */}
                            <circle
                              cx={node.x + (node.type === "scenario" ? 18 : 15)}
                              cy={node.y - (node.type === "scenario" ? 18 : 15)}
                              r="4"
                              fill={
                                node.status === "running"
                                  ? "#10b981"
                                  : node.status === "stopped"
                                  ? "#6b7280"
                                  : node.status === "starting"
                                  ? "#3b82f6"
                                  : "#f59e0b"
                              }
                              stroke="#ffffff"
                              strokeWidth="1"
                            />
                          </g>
                        );
                      })}

                      {/* Legend - moved to bottom right to avoid overlap */}
                      <g transform="translate(680, 550)">
                        <rect
                          x="0"
                          y="0"
                          width="180"
                          height="80"
                          fill="rgba(255, 255, 255, 0.9)"
                          stroke="#e5e7eb"
                          strokeWidth="1"
                          rx="4"
                          className="dark:fill-gray-800/90 dark:stroke-gray-600"
                        />
                        <text
                          x="10"
                          y="20"
                          className="text-xs fill-current text-foreground font-medium"
                        >
                          Legend
                        </text>
                        <circle
                          cx="20"
                          cy="35"
                          r="8"
                          fill="url(#scenarioGradient)"
                        />
                        <text
                          x="35"
                          y="39"
                          className="text-xs fill-current text-foreground"
                        >
                          Scenarios
                        </text>
                        <circle
                          cx="20"
                          cy="55"
                          r="6"
                          fill="url(#podGradient)"
                        />
                        <text
                          x="35"
                          y="59"
                          className="text-xs fill-current text-foreground"
                        >
                          Pods/Workloads
                        </text>
                        <text
                          x="10"
                          y="75"
                          className="text-xs fill-current text-muted-foreground"
                        >
                          Click to select, hover edges to delete
                        </text>
                      </g>
                    </svg>
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}