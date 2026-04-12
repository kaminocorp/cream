"use client";

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
} from "recharts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ChartDataPoint {
  /** Seconds since the component mounted — used as the X axis. */
  t: number;
  /** Human-readable label for the tooltip. */
  label: string;
  [key: string]: string | number;
}

interface HealthChartProps {
  data: ChartDataPoint[];
  lines: {
    key: string;
    name: string;
    color: string;
  }[];
  yLabel: string;
  yDomain?: [number, number];
  /** Format function for Y axis tick values. */
  yFormat?: (v: number) => string;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function HealthChart({
  data,
  lines,
  yLabel,
  yDomain,
  yFormat,
}: HealthChartProps) {
  if (data.length < 2) {
    return (
      <div className="flex h-[140px] items-center justify-center text-xs text-zinc-400">
        Collecting data… ({data.length}/2 samples)
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={140}>
      <LineChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
        <XAxis
          dataKey="label"
          tick={{ fontSize: 10 }}
          interval="preserveStartEnd"
          stroke="#a1a1aa"
        />
        <YAxis
          width={45}
          tick={{ fontSize: 10 }}
          domain={yDomain ?? ["auto", "auto"]}
          tickFormatter={yFormat}
          stroke="#a1a1aa"
          label={{
            value: yLabel,
            angle: -90,
            position: "insideLeft",
            style: { fontSize: 10, fill: "#a1a1aa" },
          }}
        />
        <Tooltip
          contentStyle={{
            fontSize: 12,
            borderRadius: 8,
            border: "1px solid #e4e4e7",
          }}
        />
        {lines.map((l) => (
          <Line
            key={l.key}
            type="monotone"
            dataKey={l.key}
            name={l.name}
            stroke={l.color}
            strokeWidth={1.5}
            dot={false}
            isAnimationActive={false}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}
