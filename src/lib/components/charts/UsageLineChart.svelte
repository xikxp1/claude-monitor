<script lang="ts">
import { scaleLinear, scaleTime } from "d3-scale";
import type { UsageHistoryRecord } from "$lib/historyStorage";

interface Props {
  data: UsageHistoryRecord[];
  showFiveHour?: boolean;
  showSevenDay?: boolean;
  showSonnet?: boolean;
  showOpus?: boolean;
  height?: number;
}

let {
  data,
  showFiveHour = true,
  showSevenDay = true,
  showSonnet = true,
  showOpus = true,
  height = 200,
}: Props = $props();

// Colors for each usage type
const colors = {
  fiveHour: "#3b82f6",
  sevenDay: "#8b5cf6",
  sonnet: "#22c55e",
  opus: "#f59e0b",
};

// Chart dimensions
const padding = { top: 10, right: 32, bottom: 30, left: 40 };
let containerWidth = $state(300);

let innerWidth = $derived(Math.max(0, containerWidth - padding.left - padding.right));
let innerHeight = $derived(Math.max(0, height - padding.top - padding.bottom));

// Transform data for chart
interface DataPoint {
  timestamp: Date;
  value: number;
}

function getDataPoints(records: UsageHistoryRecord[], field: keyof UsageHistoryRecord): DataPoint[] {
  return records
    .filter((r) => r[field] !== null)
    .map((r) => ({
      timestamp: new Date(r.timestamp),
      value: r[field] as number,
    }));
}

let fiveHourData = $derived(getDataPoints(data, "five_hour_utilization"));
let sevenDayData = $derived(getDataPoints(data, "seven_day_utilization"));
let sonnetData = $derived(getDataPoints(data, "sonnet_utilization"));
let opusData = $derived(getDataPoints(data, "opus_utilization"));

// Scales
let xDomain = $derived.by(() => {
  if (data.length === 0) {
    return [new Date(Date.now() - 3600000), new Date()];
  }
  const timestamps = data.map((d) => new Date(d.timestamp));
  return [
    new Date(Math.min(...timestamps.map((d) => d.getTime()))),
    new Date(Math.max(...timestamps.map((d) => d.getTime()))),
  ];
});

let xScale = $derived(scaleTime().domain(xDomain).range([0, innerWidth]));

let yScale = $derived(scaleLinear().domain([0, 100]).range([innerHeight, 0]));

// Generate path for a dataset
function generatePath(points: DataPoint[]): string {
  if (points.length === 0) return "";
  return points
    .map((p, i) => {
      const x = xScale(p.timestamp);
      const y = yScale(p.value);
      return `${i === 0 ? "M" : "L"}${x},${y}`;
    })
    .join("");
}

let fiveHourPath = $derived(generatePath(fiveHourData));
let sevenDayPath = $derived(generatePath(sevenDayData));
let sonnetPath = $derived(generatePath(sonnetData));
let opusPath = $derived(generatePath(opusData));

// Y axis ticks
let yTicks = $derived(yScale.ticks(5));

// X axis ticks
let xTicks = $derived(xScale.ticks(4));

// Format time
function formatTime(date: Date): string {
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  return `${hours}:${minutes}`;
}

let hasData = $derived(data.length > 0);

// Threshold lines
const thresholds = [
  { value: 50, color: "#eab308", label: "50%" }, // Yellow
  { value: 80, color: "#f97316", label: "80%" }, // Orange
  { value: 90, color: "#ef4444", label: "90%" }, // Red
];
</script>

<div class="chart-wrapper" bind:clientWidth={containerWidth}>
  <div class="chart-container" style="height: {height}px">
    {#if hasData}
      <svg width="100%" height="100%">
        <g transform="translate({padding.left}, {padding.top})">
          <!-- Y axis grid lines and labels -->
          {#each yTicks as tick}
            <g transform="translate(0, {yScale(tick)})">
              <line x1={0} x2={innerWidth} stroke="#e5e5e5" stroke-dasharray="2,2" class="grid-line" />
              <text x={-8} text-anchor="end" dominant-baseline="middle" class="axis-label">
                {tick}%
              </text>
            </g>
          {/each}

          <!-- X axis grid lines and labels -->
          {#each xTicks as tick}
            <g transform="translate({xScale(tick)}, 0)">
              <line y1={0} y2={innerHeight} stroke="#e5e5e5" stroke-dasharray="2,2" class="grid-line" />
              <text y={innerHeight + 16} text-anchor="middle" class="axis-label">
                {formatTime(tick)}
              </text>
            </g>
          {/each}

          <!-- Threshold lines -->
          {#each thresholds as threshold}
            <g transform="translate(0, {yScale(threshold.value)})">
              <line x1={0} x2={innerWidth} stroke={threshold.color} stroke-width="1" stroke-dasharray="4,3" opacity="0.7" />
              <text x={innerWidth + 4} dominant-baseline="middle" class="threshold-label" fill={threshold.color}>
                {threshold.label}
              </text>
            </g>
          {/each}

          <!-- Data lines -->
          {#if showFiveHour && fiveHourPath}
            <path d={fiveHourPath} fill="none" stroke={colors.fiveHour} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
          {/if}
          {#if showSevenDay && sevenDayPath}
            <path d={sevenDayPath} fill="none" stroke={colors.sevenDay} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
          {/if}
          {#if showSonnet && sonnetPath}
            <path d={sonnetPath} fill="none" stroke={colors.sonnet} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
          {/if}
          {#if showOpus && opusPath}
            <path d={opusPath} fill="none" stroke={colors.opus} stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
          {/if}
        </g>
      </svg>
    {:else}
      <div class="no-data">
        <span>No historical data yet</span>
        <span class="hint">Data will appear after a few refreshes</span>
      </div>
    {/if}
  </div>

  <div class="legend">
    {#if showFiveHour}
      <div class="legend-item">
        <span class="legend-color" style="background: {colors.fiveHour}"></span>
        <span>5 Hour</span>
      </div>
    {/if}
    {#if showSevenDay}
      <div class="legend-item">
        <span class="legend-color" style="background: {colors.sevenDay}"></span>
        <span>7 Day</span>
      </div>
    {/if}
    {#if showSonnet}
      <div class="legend-item">
        <span class="legend-color" style="background: {colors.sonnet}"></span>
        <span>Sonnet</span>
      </div>
    {/if}
    {#if showOpus}
      <div class="legend-item">
        <span class="legend-color" style="background: {colors.opus}"></span>
        <span>Opus</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .chart-wrapper {
    width: 100%;
  }

  .chart-container {
    width: 100%;
    position: relative;
  }

  .no-data {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #888;
    font-size: 0.9rem;
    gap: 4px;
  }

  .no-data .hint {
    font-size: 0.75rem;
    color: #aaa;
  }

  .grid-line {
    stroke: #e5e5e5;
  }

  .axis-label {
    fill: #888;
    font-size: 10px;
  }

  .threshold-label {
    font-size: 9px;
    font-weight: 500;
  }

  .legend {
    display: flex;
    justify-content: center;
    gap: 16px;
    margin-top: 8px;
    flex-wrap: wrap;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    color: #666;
  }

  .legend-color {
    width: 12px;
    height: 3px;
    border-radius: 2px;
  }

  @media (prefers-color-scheme: dark) {
    .grid-line {
      stroke: #3a3a3a;
    }

    .axis-label {
      fill: #888;
    }

    .legend-item {
      color: #aaa;
    }

    .no-data {
      color: #888;
    }

    .no-data .hint {
      color: #666;
    }
  }
</style>
