import React from 'react';
export const ResponsiveContainer = (props: any) => <div data-testid="recharts-responsive">{props.children}</div>;
export const PieChart = (props: any) => <div data-testid="recharts-pie">{props.children}</div>;
export const Pie = (props: any) => <div data-testid="recharts-pie-elem" />;
export const Cell = (props: any) => <div data-testid="recharts-cell" />;
export default {
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
};
