import React from "react";
import { render, screen } from "@testing-library/react";
import { Overview } from "../components/Overview";

describe("Overview component", () => {
  it("renders cluster overview title", () => {
    render(<Overview />);
    expect(screen.getByText(/PULLPIRI Cluster Overview/i)).toBeInTheDocument();
  });

  it("renders stat cards with correct values", () => {
    render(<Overview />);
    expect(screen.getByText("Nodes")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("Active Pods")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByText("Services")).toBeInTheDocument();
    expect(screen.getByText("15")).toBeInTheDocument();
    expect(screen.getByText("Deployments")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
  });

  it("renders pod distribution section", () => {
    render(<Overview />);
    expect(screen.getByText(/Pod Distribution/i)).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.getByText("Pending")).toBeInTheDocument();
    expect(screen.getByText("Failed")).toBeInTheDocument();
  });

  it("renders node performance section", () => {
    render(<Overview />);
    expect(screen.getByText(/Node Performance/i)).toBeInTheDocument();
    expect(screen.getByText(/node-1/i)).toBeInTheDocument();
    expect(screen.getByText(/node-2/i)).toBeInTheDocument();
    expect(screen.getByText(/node-3/i)).toBeInTheDocument();
  });

  it("renders resource usage trends chart section", () => {
    render(<Overview />);
    expect(screen.getByText(/Cluster Resource Usage Trends/i)).toBeInTheDocument();
    expect(screen.getByText(/Cluster CPU Usage/i)).toBeInTheDocument();
    expect(screen.getByText(/Cluster Memory Usage/i)).toBeInTheDocument();
  });
});
