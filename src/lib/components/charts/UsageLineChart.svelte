<script lang="ts">
  import type { UsageHistoryPoint } from "$lib/historyStorage";

  interface Props {
    data: UsageHistoryPoint[];
    filters: Record<string, boolean>;
    height?: number;
  }

  let { data, filters, height = 200 }: Props = $props();

  const palette = ["#3b82f6", "#8b5cf6", "#22c55e", "#f59e0b", "#ef4444", "#14b8a6"];
  const padding = { top: 10, right: 32, bottom: 30, left: 40 };
  let containerWidth = $state(300);

  let innerWidth = $derived(Math.max(0, containerWidth - padding.left - padding.right));
  let innerHeight = $derived(Math.max(0, height - padding.top - padding.bottom));

  let windowMeta = $derived.by(() => {
    const unique: Record<string, string> = {};
    for (const point of data) {
      if (!(point.windowKey in unique)) {
        unique[point.windowKey] = point.label;
      }
    }
    return Object.entries(unique).map(([key, label], index) => ({
      key,
      label,
      color: palette[index % palette.length],
    }));
  });

  function getPoints(windowKey: string) {
    return data
      .filter((point) => point.windowKey === windowKey)
      .map((point) => ({
        timestamp: new Date(point.timestamp),
        value: point.utilization,
      }));
  }

  let xDomain = $derived.by(() => {
    if (data.length === 0) {
      return [Date.now() - 3_600_000, Date.now()];
    }
    const timestamps = data.map((point) => new Date(point.timestamp).getTime());
    return [Math.min(...timestamps), Math.max(...timestamps)];
  });

  function scaleX(timestamp: Date) {
    const [start, end] = xDomain;
    if (start === end || innerWidth === 0) {
      return 0;
    }
    return ((timestamp.getTime() - start) / (end - start)) * innerWidth;
  }

  function scaleY(value: number) {
    if (innerHeight === 0) {
      return 0;
    }
    return innerHeight - (value / 100) * innerHeight;
  }

  function generatePath(windowKey: string) {
    const points = getPoints(windowKey);
    if (points.length === 0) {
      return "";
    }
    return points
      .map((point, index) => {
        const x = scaleX(point.timestamp);
        const y = scaleY(point.value);
        return `${index === 0 ? "M" : "L"}${x},${y}`;
      })
      .join("");
  }

  let yTicks = $derived([0, 25, 50, 75, 100]);
  let xTicks = $derived.by(() => {
    const [start, end] = xDomain;
    const segments = 4;
    const step = (end - start) / segments;
    return Array.from({ length: segments + 1 }, (_, index) => new Date(start + step * index));
  });

  function formatTime(date: Date): string {
    const hours = date.getHours().toString().padStart(2, "0");
    const minutes = date.getMinutes().toString().padStart(2, "0");
    return `${hours}:${minutes}`;
  }

  let hasData = $derived(data.length > 0);
</script>

<div class="chart-wrapper" bind:clientWidth={containerWidth}>
  <div class="chart-container" style="height: {height}px">
    {#if hasData}
      <svg width="100%" height="100%">
        <g transform="translate({padding.left}, {padding.top})">
          {#each yTicks as tick (tick)}
            <g transform="translate(0, {scaleY(tick)})">
              <line
                x1={0}
                x2={innerWidth}
                stroke="#e5e5e5"
                stroke-dasharray="2,2"
                class="grid-line"
              />
              <text x={-8} text-anchor="end" dominant-baseline="middle" class="axis-label">
                {tick}%
              </text>
            </g>
          {/each}

          {#each xTicks as tick (tick.getTime())}
            <g transform="translate({scaleX(tick)}, 0)">
              <line
                y1={0}
                y2={innerHeight}
                stroke="#e5e5e5"
                stroke-dasharray="2,2"
                class="grid-line"
              />
              <text y={innerHeight + 16} text-anchor="middle" class="axis-label">
                {formatTime(tick)}
              </text>
            </g>
          {/each}

          {#each windowMeta as window (window.key)}
            {#if filters[window.key] !== false}
              {@const path = generatePath(window.key)}
              {#if path}
                <path
                  d={path}
                  fill="none"
                  stroke={window.color}
                  stroke-width="2"
                  stroke-linejoin="round"
                  stroke-linecap="round"
                />
              {/if}
            {/if}
          {/each}
        </g>
      </svg>
    {:else}
      <div class="no-data">
        <span>No historical data yet</span>
        <span class="hint">Data will appear after a few refreshes</span>
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
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    color: color-mix(in srgb, currentColor 60%, transparent);
    font-size: 0.875rem;
  }

  .hint {
    font-size: 0.75rem;
  }

  .axis-label {
    font-size: 10px;
    fill: color-mix(in srgb, currentColor 65%, transparent);
  }

  .grid-line {
    opacity: 0.5;
  }
</style>
