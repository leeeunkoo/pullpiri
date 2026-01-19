// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0
import { useState, useRef/*, useEffect */} from "react"; // 2025-09-23 comment out
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { 
  GitBranch, 
  Box, 
  Package, 
  Activity,
  ZoomIn,
  ZoomOut,
  RotateCcw,
  //Maximize2, // 2025-09-23 comment out
  Search,
} from "lucide-react";
import { Input } from "./ui/input";

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

interface DependenciesProps {
  namespace: string;
}

export function Dependencies({ namespace }: DependenciesProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState(1);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  // Define nodes and edges data
  const nodes: Node[] = [
    // Scenarios
    {
      id: "ad_driving",
      label: "AD Driving",
      type: "scenario", 
      status: "running",
      x: 200,
      y: 100,
      color: "#3b82f6"
    },
    {
      id: "manual_driving", 
      label: "Manual Driving",
      type: "scenario",
      status: "stopped", 
      x: 200,
      y: 300,
      color: "#6b7280"
    },
    {
      id: "emergency_mode",
      label: "Emergency Mode", 
      type: "scenario",
      status: "starting",
      x: 200,
      y: 500,
      color: "#f59e0b"
    },
    // Pods
    {
      id: "autonomous_algo",
      label: "자동주행알고리즘",
      type: "pod",
      status: "running",
      x: 500,
      y: 80,
      color: "#10b981"
    },
    {
      id: "vehicle_monitor",
      label: "차량상태모니터링", 
      type: "pod",
      status: "running",
      x: 600,
      y: 250,
      color: "#10b981"
    },
    {
      id: "driver_assist",
      label: "운전자보조시스템",
      type: "pod", 
      status: "stopped",
      x: 500,
      y: 320,
      color: "#6b7280"
    },
    {
      id: "emergency_brake",
      label: "응급제동시스템",
      type: "pod",
      status: "starting", 
      x: 500,
      y: 480,
      color: "#f59e0b"
    },
    {
      id: "emergency_comm",
      label: "비상통신모듈",
      type: "pod",
      status: "pending",
      x: 500,
      y: 520,
      color: "#f97316"
    }
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
    { from: "emergency_mode", to: "vehicle_monitor" }
  ];

  const filteredNodes = nodes.filter(node => 
    node.label.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const filteredEdges = edges.filter(edge => {
    const fromNode = filteredNodes.find(n => n.id === edge.from);
    const toNode = filteredNodes.find(n => n.id === edge.to);
    return fromNode && toNode;
  });

  const getNodeColor = (node: Node) => {
    if (selectedNode === node.id) {
      return "#dc2626"; // Red for selected
    }
    return node.color;
  };

  const getStatusBadgeColor = (status: string) => {
    switch (status) {
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

  const handleZoomIn = () => {
    setZoom(prev => Math.min(prev * 1.2, 3));
  };

  const handleZoomOut = () => {
    setZoom(prev => Math.max(prev / 1.2, 0.3));
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

  // Calculate connected nodes for highlighting
  const getConnectedNodes = (nodeId: string) => {
    const connected = new Set<string>();
    edges.forEach(edge => {
      if (edge.from === nodeId) connected.add(edge.to);
      if (edge.to === nodeId) connected.add(edge.from);
    });
    return connected;
  };

  const connectedNodes = selectedNode ? getConnectedNodes(selectedNode) : new Set<string>();

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="mb-2">Dependencies</h1>
          <p className="text-muted-foreground">
            Visual representation of scenario and pod dependencies
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="secondary" className="gap-1">
            <Package className="w-3 h-3" />
            Namespace: {namespace}
          </Badge>
        </div>
      </div>

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
              <Button variant="outline" size="sm" onClick={handleZoomIn}>
                <ZoomIn className="w-4 h-4" />
              </Button>
              <Button variant="outline" size="sm" onClick={handleZoomOut}>
                <ZoomOut className="w-4 h-4" />
              </Button>
              <Button variant="outline" size="sm" onClick={handleReset}>
                <RotateCcw className="w-4 h-4" />
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Dependency Graph */}
      <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GitBranch className="w-5 h-5 text-primary" />
            Dependency Graph
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="relative w-full h-[600px] bg-muted/20 rounded-lg border border-border/30 overflow-hidden">
            <svg
              ref={svgRef}
              width="100%"
              height="100%"
              className="cursor-grab active:cursor-grabbing"
              style={{
                transform: `translate(${panX}px, ${panY}px) scale(${zoom})`
              }}
            >
              {/* Grid Pattern */}
              <defs>
                <pattern id="grid" width="20" height="20" patternUnits="userSpaceOnUse">
                  <path
                    d="M 20 0 L 0 0 0 20"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="0.5"
                    className="text-border/20"
                  />
                </pattern>
              </defs>
              <rect width="100%" height="100%" fill="url(#grid)" />

              {/* Edges */}
              {filteredEdges.map((edge, index) => {
                const fromNode = filteredNodes.find(n => n.id === edge.from);
                const toNode = filteredNodes.find(n => n.id === edge.to);
                
                if (!fromNode || !toNode) return null;

                const isHighlighted = selectedNode && 
                  (edge.from === selectedNode || edge.to === selectedNode);

                return (
                  <line
                    key={index}
                    x1={fromNode.x}
                    y1={fromNode.y}
                    x2={toNode.x}
                    y2={toNode.y}
                    stroke={isHighlighted ? "#dc2626" : "#6b7280"}
                    strokeWidth={isHighlighted ? 3 : 2}
                    strokeOpacity={isHighlighted ? 1 : 0.6}
                    markerEnd="url(#arrowhead)"
                  />
                );
              })}

              {/* Arrow marker */}
              <defs>
                <marker
                  id="arrowhead"
                  markerWidth="10"
                  markerHeight="7"
                  refX="9"
                  refY="3.5"
                  orient="auto"
                >
                  <polygon
                    points="0 0, 10 3.5, 0 7"
                    fill="#6b7280"
                  />
                </marker>
              </defs>

              {/* Nodes */}
              {filteredNodes.map((node) => {
                const isConnected = selectedNode && connectedNodes.has(node.id);
                const isSelected = selectedNode === node.id;
                const opacity = selectedNode && !isSelected && !isConnected ? 0.3 : 1;

                return (
                  <g key={node.id} style={{ opacity }}>
                    {/* Node circle */}
                    <circle
                      cx={node.x}
                      cy={node.y}
                      r={node.type === "scenario" ? 20 : 15}
                      fill={getNodeColor(node)}
                      stroke={isSelected ? "#dc2626" : "#ffffff"}
                      strokeWidth={isSelected ? 3 : 2}
                      className="cursor-pointer hover:stroke-primary transition-all duration-200"
                      onClick={() => handleNodeClick(node.id)}
                    />
                    
                    {/* Node icon */}
                    {node.type === "scenario" ? (
                      <text
                        x={node.x}
                        y={node.y + 2}
                        textAnchor="middle"
                        fill="white"
                        fontSize="12"
                        className="pointer-events-none select-none"
                      >
                        S
                      </text>
                    ) : (
                      <text
                        x={node.x}
                        y={node.y + 2}
                        textAnchor="middle"
                        fill="white"
                        fontSize="10"
                        className="pointer-events-none select-none"
                      >
                        P
                      </text>
                    )}
                    
                    {/* Node label */}
                    <text
                      x={node.x}
                      y={node.y + (node.type === "scenario" ? 35 : 30)}
                      textAnchor="middle"
                      className="fill-foreground text-xs pointer-events-none select-none"
                      style={{ fontSize: '11px' }}
                    >
                      {node.label}
                    </text>
                  </g>
                );
              })}
            </svg>

            {/* Zoom indicator */}
            <div className="absolute top-4 right-4 bg-card/70 backdrop-blur-sm rounded-lg p-2 border border-border/30">
              <span className="text-xs text-muted-foreground">
                Zoom: {Math.round(zoom * 100)}%
              </span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Legend and Node Details */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Legend */}
        <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              Legend
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center gap-3">
                <div className="w-6 h-6 rounded-full bg-primary flex items-center justify-center">
                  <span className="text-white text-xs">S</span>
                </div>
                <span className="text-sm">Scenario</span>
              </div>
              <div className="flex items-center gap-3">
                <div className="w-5 h-5 rounded-full bg-chart-1 flex items-center justify-center">
                  <span className="text-white text-xs">P</span>
                </div>
                <span className="text-sm">Pod</span>
              </div>
            </div>
            
            <div className="pt-2 border-t border-border/20">
              <p className="text-xs text-muted-foreground mb-2">Status Colors:</p>
              <div className="grid grid-cols-2 gap-2 text-xs">
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-emerald-500"></div>
                  <span>Running</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-gray-500"></div>
                  <span>Stopped</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-blue-500"></div>
                  <span>Starting</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-3 h-3 rounded-full bg-amber-500"></div>
                  <span>Pending</span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Selected Node Details */}
        <Card className="bg-card/50 backdrop-blur-sm border-border/50 shadow-lg">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="w-5 h-5 text-primary" />
              Node Details
            </CardTitle>
          </CardHeader>
          <CardContent>
            {selectedNode ? (
              <div className="space-y-3">
                {(() => {
                  const node = nodes.find(n => n.id === selectedNode);
                  if (!node) return null;
                  
                  return (
                    <>
                      <div className="flex items-center justify-between">
                        <h4 className="font-medium">{node.label}</h4>
                        <Badge 
                          variant="outline" 
                          className={getStatusBadgeColor(node.status)}
                        >
                          {node.status.charAt(0).toUpperCase() + node.status.slice(1)}
                        </Badge>
                      </div>
                      
                      <div className="space-y-2 text-sm">
                        <div>
                          <span className="text-muted-foreground">Type: </span>
                          <span className="capitalize">{node.type}</span>
                        </div>
                        
                        <div>
                          <span className="text-muted-foreground">Connected to: </span>
                          <div className="mt-1 flex flex-wrap gap-1">
                            {Array.from(connectedNodes).map(nodeId => {
                              const connectedNode = nodes.find(n => n.id === nodeId);
                              return connectedNode ? (
                                <Badge key={nodeId} variant="secondary" className="text-xs">
                                  {connectedNode.label}
                                </Badge>
                              ) : null;
                            })}
                          </div>
                        </div>
                      </div>
                    </>
                  );
                })()}
              </div>
            ) : (
              <div className="text-center text-muted-foreground py-8">
                <Box className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>Click on a node to view details</p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}