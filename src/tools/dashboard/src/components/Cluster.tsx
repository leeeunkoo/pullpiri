// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0
import { useState } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "./ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "./ui/table";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import {
  Search,
  MoreHorizontal,
  Plus,
  Server,
  Cpu,
  MemoryStick,
  HardDrive,
} from "lucide-react";

export function Cluster() {
  const [searchTerm, setSearchTerm] = useState("");

  // Mock data
  const nodes = [
    {
      name: "master-node-1",
      internalIP: "192.168.1.10",
      os: "Ubuntu 22.04",
      arch: "amd64",
      cpuCapacity: "4",
      memoryCapacity: "8Gi",
      storage: "50Gi",
    },
    {
      name: "worker-node-1",
      internalIP: "192.168.1.11",
      os: "Ubuntu 22.04",
      arch: "amd64",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      storage: "100Gi",
    },
    {
      name: "worker-node-2",
      internalIP: "192.168.1.12",
      os: "Ubuntu 22.04",
      arch: "amd64",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      storage: "100Gi",
    },
    {
      name: "worker-node-3",
      internalIP: "192.168.1.13",
      os: "Ubuntu 22.04",
      arch: "arm64",
      cpuCapacity: "8",
      memoryCapacity: "16Gi",
      storage: "100Gi",
    },
  ];



  const filteredNodes = nodes.filter((node) =>
    node.name.toLowerCase().includes(searchTerm.toLowerCase()),
  );

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div className="relative">
          <div className="flex items-center gap-4 mb-2">
            <div className="w-1 h-8 bg-gradient-to-b from-primary to-primary/80 rounded-full"></div>
            <h1 className="font-bold text-foreground text-[20px]">
              PULLPIRI Nodes
            </h1>
          </div>
          <p className="text-muted-foreground ml-8">
            Monitor and manage your cluster nodes infrastructure
          </p>
        </div>
        <Button className="bg-primary hover:bg-primary/80 text-primary-foreground shadow-lg hover:shadow-xl transition-all gap-2">
          <Plus className="w-4 h-4" />
          Add Node
        </Button>
      </div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search nodes..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10 h-12 bg-card/80 backdrop-blur-sm border-border/30 shadow-sm hover:shadow-md transition-all"
          />
        </div>
        <div className="flex items-center gap-3">
          <Badge className="bg-slate-50 dark:bg-slate-950 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-800 px-3 py-1">
            <Server className="w-3 h-3 mr-1" />
            {nodes.length} Nodes
          </Badge>
        </div>
      </div>

      {/* Nodes Table */}
      <Card className="bg-card/80 backdrop-blur-sm border-border/20 shadow-xl">
        <CardHeader>
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
              <Server className="w-4 h-4 text-primary-foreground" />
            </div>
            <div>
              <CardTitle className="text-foreground">
                Cluster Nodes
              </CardTitle>
              <CardDescription>
                Physical and virtual machines in your cluster
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="overflow-hidden rounded-xl border border-border/30">
            <Table>
              <TableHeader className="bg-muted/80">
                <TableRow className="border-border/30">
                  <TableHead className="font-semibold text-foreground">
                    Name
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    OS
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Arch
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Internal IP
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    CPU
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Memory
                  </TableHead>
                  <TableHead className="font-semibold text-foreground">
                    Storage
                  </TableHead>
                  <TableHead className="font-semibold text-foreground"></TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredNodes.map((node) => (
                  <TableRow
                    key={node.name}
                    className="border-border/30 hover:bg-muted/30 transition-colors"
                  >
                    <TableCell className="font-medium text-foreground">
                      {node.name}
                    </TableCell>
                    <TableCell className="text-sm">
                      {node.os}
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="text-xs">
                        {node.arch}
                      </Badge>
                    </TableCell>
                    <TableCell className="font-mono text-sm text-muted-foreground">
                      {node.internalIP}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Cpu className="w-3 h-3 text-muted-foreground" />
                        <span className="font-mono text-sm">
                          {node.cpuCapacity}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <MemoryStick className="w-3 h-3 text-muted-foreground" />
                        <span className="font-mono text-sm">
                          {node.memoryCapacity}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <HardDrive className="w-3 h-3 text-muted-foreground" />
                        <span className="font-mono text-sm">
                          {node.storage}
                        </span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="w-8 h-8 hover:bg-muted"
                      >
                        <MoreHorizontal className="h-3 w-3" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}