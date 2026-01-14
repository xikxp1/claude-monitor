<script lang="ts">
  import type {
    NotificationRule,
    NotificationSettings,
    UsageType,
  } from "$lib/types";
  import { USAGE_TYPE_LABELS } from "$lib/types";

  interface Props {
    settings: NotificationSettings;
    onchange: (settings: NotificationSettings) => void;
  }

  let { settings, onchange }: Props = $props();

  let openSection: UsageType | null = $state(null);

  function toggleSection(usageType: UsageType) {
    openSection = openSection === usageType ? null : usageType;
  }

  function updateRule(
    usageType: UsageType,
    updates: Partial<NotificationRule>,
  ) {
    const newSettings = {
      ...settings,
      [usageType]: { ...settings[usageType], ...updates },
    };
    onchange(newSettings);
  }

  // Predefined threshold options in percent
  const THRESHOLD_OPTIONS = [50, 80, 90, 95];

  function toggleThreshold(usageType: UsageType, threshold: number) {
    const rule = settings[usageType];
    const current = rule.thresholds;
    const newThresholds = current.includes(threshold)
      ? current.filter((t) => t !== threshold)
      : [...current, threshold].sort((a, b) => a - b);
    updateRule(usageType, { thresholds: newThresholds });
  }

  // Predefined time options in minutes (short for 5h, extended for 7d)
  const TIME_OPTIONS_SHORT = [
    { value: 15, label: "15m" },
    { value: 30, label: "30m" },
    { value: 60, label: "1h" },
    { value: 120, label: "2h" },
  ];

  const TIME_OPTIONS_EXTENDED = [
    { value: 30, label: "30m" },
    { value: 60, label: "1h" },
    { value: 120, label: "2h" },
    { value: 240, label: "4h" },
    { value: 720, label: "12h" },
    { value: 1440, label: "1d" },
    { value: 2880, label: "2d" },
  ];

  function getTimeOptionsForUsageType(usageType: UsageType) {
    return usageType === "five_hour" ? TIME_OPTIONS_SHORT : TIME_OPTIONS_EXTENDED;
  }

  function toggleTimeOption(usageType: UsageType, minutes: number) {
    const rule = settings[usageType];
    const current = rule.time_remaining_minutes;
    const newMinutes = current.includes(minutes)
      ? current.filter((m) => m !== minutes)
      : [...current, minutes].sort((a, b) => a - b);
    updateRule(usageType, { time_remaining_minutes: newMinutes });
  }

  function formatTimeMinutes(minutes: number[]): string {
    return minutes
      .map((m) => {
        if (m >= 60) {
          const hours = Math.floor(m / 60);
          const mins = m % 60;
          return mins > 0 ? `${hours}h${mins}m` : `${hours}h`;
        }
        return `${m}m`;
      })
      .join(", ");
  }

  function toggleEnabled() {
    onchange({ ...settings, enabled: !settings.enabled });
  }

  function getRuleSummary(rule: NotificationRule): string {
    const parts: string[] = [];
    if (rule.interval_enabled) parts.push(`every ${rule.interval_percent}%`);
    if (rule.threshold_enabled && rule.thresholds.length > 0) {
      parts.push(`at ${rule.thresholds.join(", ")}%`);
    }
    if (rule.time_remaining_enabled && rule.time_remaining_minutes.length > 0) {
      parts.push(`< ${formatTimeMinutes(rule.time_remaining_minutes)}`);
    }
    return parts.length > 0 ? parts.join(", ") : "off";
  }

  const usageTypes: UsageType[] = [
    "five_hour",
    "seven_day",
    "seven_day_sonnet",
    "seven_day_opus",
  ];
</script>

<div class="flex flex-col gap-2">
  <label class="flex items-center gap-2 cursor-pointer font-medium pb-2 border-b border-base-300">
    <input
      type="checkbox"
      class="checkbox checkbox-primary checkbox-sm"
      checked={settings.enabled}
      onchange={toggleEnabled}
    />
    <span>Enable notifications</span>
  </label>

  {#if settings.enabled}
    <div class="flex flex-col gap-1">
      {#each usageTypes as usageType}
        {@const rule = settings[usageType]}
        {@const isOpen = openSection === usageType}
        <div class="bg-base-200 rounded-md overflow-hidden">
          <button
            type="button"
            class="flex items-center gap-1.5 w-full px-2.5 py-2 bg-transparent border-none cursor-pointer text-left text-[0.85rem] hover:bg-base-300/50"
            onclick={() => toggleSection(usageType)}
          >
            <span class="text-[0.65rem] w-2.5 text-base-content/50">{isOpen ? "▼" : "▶"}</span>
            <span class="font-medium">{USAGE_TYPE_LABELS[usageType]}</span>
            <span class="ml-auto text-xs text-base-content/50">{getRuleSummary(rule)}</span>
          </button>
          {#if isOpen}
            <div class="px-2.5 pb-2.5 pl-6 flex flex-col gap-1.5">
              <label class="flex items-center gap-1.5 text-[0.8rem] cursor-pointer">
                <input
                  type="checkbox"
                  class="checkbox checkbox-primary checkbox-xs"
                  checked={rule.interval_enabled}
                  onchange={() =>
                    updateRule(usageType, {
                      interval_enabled: !rule.interval_enabled,
                    })}
                />
                <span>Every</span>
                <select
                  class="select select-bordered select-xs"
                  value={rule.interval_percent}
                  disabled={!rule.interval_enabled}
                  onchange={(e) =>
                    updateRule(usageType, {
                      interval_percent: Number.parseInt(
                        e.currentTarget.value,
                        10,
                      ),
                    })}
                >
                  <option value={5}>5%</option>
                  <option value={10}>10%</option>
                  <option value={15}>15%</option>
                  <option value={20}>20%</option>
                  <option value={25}>25%</option>
                </select>
              </label>

              <div class="flex flex-col gap-1.5">
                <label class="flex items-center gap-1.5 text-[0.8rem] cursor-pointer">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-primary checkbox-xs"
                    checked={rule.threshold_enabled}
                    onchange={() =>
                      updateRule(usageType, {
                        threshold_enabled: !rule.threshold_enabled,
                      })}
                  />
                  <span>At thresholds:</span>
                </label>
                <div class="flex flex-wrap gap-1 ml-5 {!rule.threshold_enabled ? 'opacity-50' : ''}">
                  {#each THRESHOLD_OPTIONS as threshold}
                    <button
                      type="button"
                      class="btn btn-xs {rule.thresholds.includes(threshold) ? 'btn-primary' : 'btn-ghost'}"
                      disabled={!rule.threshold_enabled}
                      onclick={() => toggleThreshold(usageType, threshold)}
                    >
                      {threshold}%
                    </button>
                  {/each}
                </div>
              </div>

              <div class="flex flex-col gap-1.5">
                <label class="flex items-center gap-1.5 text-[0.8rem] cursor-pointer">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-primary checkbox-xs"
                    checked={rule.time_remaining_enabled}
                    onchange={() =>
                      updateRule(usageType, {
                        time_remaining_enabled: !rule.time_remaining_enabled,
                      })}
                  />
                  <span>Before reset:</span>
                </label>
                <div class="flex flex-wrap gap-1 ml-5 {!rule.time_remaining_enabled ? 'opacity-50' : ''}">
                  {#each getTimeOptionsForUsageType(usageType) as option}
                    <button
                      type="button"
                      class="btn btn-xs {rule.time_remaining_minutes.includes(option.value) ? 'btn-primary' : 'btn-ghost'}"
                      disabled={!rule.time_remaining_enabled}
                      onclick={() => toggleTimeOption(usageType, option.value)}
                    >
                      {option.label}
                    </button>
                  {/each}
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
