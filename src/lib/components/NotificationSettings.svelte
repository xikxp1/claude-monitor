<script lang="ts">
  import type {
    NotificationRule,
    NotificationSettings,
    ProviderKind,
    UsageWindow,
  } from "$lib/types";
  import { getDefaultNotificationRule, getWindowRuleKey } from "$lib/types";

  interface Props {
    settings: NotificationSettings;
    provider: ProviderKind;
    windows: UsageWindow[];
    onchange: (settings: NotificationSettings) => void;
  }

  let { settings, provider, windows, onchange }: Props = $props();

  let openSection: string | null = $state(null);

  const THRESHOLD_OPTIONS = [50, 80, 90, 95];
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

  function toggleSection(windowKey: string) {
    openSection = openSection === windowKey ? null : windowKey;
  }

  function getRule(windowKey: string): NotificationRule {
    return settings.rules[getWindowRuleKey(provider, windowKey)] ?? getDefaultNotificationRule();
  }

  function updateRule(windowKey: string, updates: Partial<NotificationRule>) {
    const ruleKey = getWindowRuleKey(provider, windowKey);
    const nextSettings = {
      ...settings,
      rules: {
        ...settings.rules,
        [ruleKey]: {
          ...getRule(windowKey),
          ...updates,
        },
      },
    };
    onchange(nextSettings);
  }

  function toggleThreshold(windowKey: string, threshold: number) {
    const rule = getRule(windowKey);
    const thresholds = rule.thresholds.includes(threshold)
      ? rule.thresholds.filter((value) => value !== threshold)
      : [...rule.thresholds, threshold].sort((a, b) => a - b);
    updateRule(windowKey, { thresholds });
  }

  function getTimeOptions(window: UsageWindow) {
    const seconds = window.windowDurationSeconds ?? 0;
    return seconds > 0 && seconds <= 21_600 ? TIME_OPTIONS_SHORT : TIME_OPTIONS_EXTENDED;
  }

  function toggleTimeOption(windowKey: string, minutes: number) {
    const rule = getRule(windowKey);
    const timeRemaining = rule.time_remaining_minutes.includes(minutes)
      ? rule.time_remaining_minutes.filter((value) => value !== minutes)
      : [...rule.time_remaining_minutes, minutes].sort((a, b) => a - b);
    updateRule(windowKey, { time_remaining_minutes: timeRemaining });
  }

  function formatTimeMinutes(minutes: number[]): string {
    return minutes
      .map((value) => {
        if (value >= 60) {
          const hours = Math.floor(value / 60);
          const remainder = value % 60;
          return remainder > 0 ? `${hours}h${remainder}m` : `${hours}h`;
        }
        return `${value}m`;
      })
      .join(", ");
  }

  function getRuleSummary(windowKey: string): string {
    const rule = getRule(windowKey);
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
</script>

<div class="flex flex-col gap-2">
  <label class="flex items-center gap-2 cursor-pointer font-medium pb-2 border-b border-base-300">
    <input
      type="checkbox"
      class="checkbox checkbox-primary checkbox-sm"
      checked={settings.enabled}
      onchange={() => onchange({ ...settings, enabled: !settings.enabled })}
    />
    <span>Enable notifications</span>
  </label>

  {#if settings.enabled}
    <div class="flex flex-col gap-1">
      {#each windows as window (window.key)}
        {@const rule = getRule(window.key)}
        {@const isOpen = openSection === window.key}
        <div class="bg-base-200 rounded-md overflow-hidden">
          <button
            type="button"
            class="flex items-center gap-1.5 w-full px-2.5 py-2 bg-transparent border-none cursor-pointer text-left text-[0.85rem] hover:bg-base-300/50"
            onclick={() => toggleSection(window.key)}
          >
            <span class="text-[0.65rem] w-2.5 text-base-content/50">{isOpen ? "▼" : "▶"}</span>
            <span class="font-medium">{window.label}</span>
            <span class="ml-auto text-xs text-base-content/50">{getRuleSummary(window.key)}</span>
          </button>
          {#if isOpen}
            <div class="px-2.5 pb-2.5 pl-6 flex flex-col gap-1.5">
              <label class="flex items-center gap-1.5 text-[0.8rem] cursor-pointer">
                <input
                  type="checkbox"
                  class="checkbox checkbox-primary checkbox-xs"
                  checked={rule.interval_enabled}
                  onchange={() =>
                    updateRule(window.key, {
                      interval_enabled: !rule.interval_enabled,
                    })}
                />
                <span>Every</span>
                <select
                  class="select select-bordered select-xs"
                  value={rule.interval_percent}
                  disabled={!rule.interval_enabled}
                  onchange={(event) =>
                    updateRule(window.key, {
                      interval_percent: Number.parseInt(event.currentTarget.value, 10),
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
                      updateRule(window.key, {
                        threshold_enabled: !rule.threshold_enabled,
                      })}
                  />
                  <span>At thresholds:</span>
                </label>
                <div class="flex flex-wrap gap-1 ml-5 {!rule.threshold_enabled ? 'opacity-50' : ''}">
                  {#each THRESHOLD_OPTIONS as threshold (threshold)}
                    <button
                      type="button"
                      class="btn btn-xs {rule.thresholds.includes(threshold) ? 'btn-primary' : 'btn-ghost'}"
                      disabled={!rule.threshold_enabled}
                      onclick={() => toggleThreshold(window.key, threshold)}
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
                      updateRule(window.key, {
                        time_remaining_enabled: !rule.time_remaining_enabled,
                      })}
                  />
                  <span>Before reset:</span>
                </label>
                <div class="flex flex-wrap gap-1 ml-5 {!rule.time_remaining_enabled ? 'opacity-50' : ''}">
                  {#each getTimeOptions(window) as option (option.value)}
                    <button
                      type="button"
                      class="btn btn-xs {rule.time_remaining_minutes.includes(option.value) ? 'btn-primary' : 'btn-ghost'}"
                      disabled={!rule.time_remaining_enabled}
                      onclick={() => toggleTimeOption(window.key, option.value)}
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
