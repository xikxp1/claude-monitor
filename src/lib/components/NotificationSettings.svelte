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

<div class="notification-settings">
  <label class="global-toggle">
    <input
      type="checkbox"
      checked={settings.enabled}
      onchange={toggleEnabled}
    />
    <span>Enable notifications</span>
  </label>

  {#if settings.enabled}
    <div class="rules-container">
      {#each usageTypes as usageType}
        {@const rule = settings[usageType]}
        {@const isOpen = openSection === usageType}
        <div class="rule-section" class:open={isOpen}>
          <button
            type="button"
            class="rule-header"
            onclick={() => toggleSection(usageType)}
          >
            <span class="arrow">{isOpen ? "▼" : "▶"}</span>
            <span class="title">{USAGE_TYPE_LABELS[usageType]}</span>
            <span class="summary">{getRuleSummary(rule)}</span>
          </button>
          {#if isOpen}
            <div class="rule-content">
              <label class="inline-option">
                <input
                  type="checkbox"
                  checked={rule.interval_enabled}
                  onchange={() =>
                    updateRule(usageType, {
                      interval_enabled: !rule.interval_enabled,
                    })}
                />
                <span>Every</span>
                <select
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

              <div class="threshold-option">
                <label class="inline-option">
                  <input
                    type="checkbox"
                    checked={rule.threshold_enabled}
                    onchange={() =>
                      updateRule(usageType, {
                        threshold_enabled: !rule.threshold_enabled,
                      })}
                  />
                  <span>At thresholds:</span>
                </label>
                <div class="threshold-chips" class:disabled={!rule.threshold_enabled}>
                  {#each THRESHOLD_OPTIONS as threshold}
                    <button
                      type="button"
                      class="threshold-chip"
                      class:selected={rule.thresholds.includes(threshold)}
                      disabled={!rule.threshold_enabled}
                      onclick={() => toggleThreshold(usageType, threshold)}
                    >
                      {threshold}%
                    </button>
                  {/each}
                </div>
              </div>

              <div class="time-remaining-option">
                <label class="inline-option">
                  <input
                    type="checkbox"
                    checked={rule.time_remaining_enabled}
                    onchange={() =>
                      updateRule(usageType, {
                        time_remaining_enabled: !rule.time_remaining_enabled,
                      })}
                  />
                  <span>Before reset:</span>
                </label>
                <div class="time-chips" class:disabled={!rule.time_remaining_enabled}>
                  {#each getTimeOptionsForUsageType(usageType) as option}
                    <button
                      type="button"
                      class="time-chip"
                      class:selected={rule.time_remaining_minutes.includes(option.value)}
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

<style>
  .notification-settings {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .global-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: 500;
    padding-bottom: 8px;
    border-bottom: 1px solid #e0e0e0;
  }

  @media (prefers-color-scheme: dark) {
    .global-toggle {
      border-bottom-color: #333;
    }
  }

  .global-toggle input[type="checkbox"] {
    width: 14px;
    height: 14px;
    accent-color: #7c3aed;
  }

  .rules-container {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .rule-section {
    background: #f5f5f5;
    border-radius: 6px;
    overflow: hidden;
  }

  @media (prefers-color-scheme: dark) {
    .rule-section {
      background: #2a2a2a;
    }
  }

  .rule-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    background: transparent;
    cursor: pointer;
    text-align: left;
    font-size: 0.85rem;
    color: inherit;
  }

  .rule-header:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  @media (prefers-color-scheme: dark) {
    .rule-header:hover {
      background: rgba(255, 255, 255, 0.05);
    }
  }

  .arrow {
    font-size: 0.65rem;
    width: 10px;
    color: #888;
  }

  .title {
    font-weight: 500;
  }

  .summary {
    margin-left: auto;
    font-size: 0.75rem;
    color: #888;
  }

  .rule-content {
    padding: 4px 10px 10px 26px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .inline-option {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .inline-option input[type="checkbox"] {
    width: 14px;
    height: 14px;
    accent-color: #7c3aed;
  }

  .inline-option select {
    padding: 4px 6px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.8rem;
    background: #fff;
  }

  .inline-option select:disabled {
    opacity: 0.5;
  }

  @media (prefers-color-scheme: dark) {
    .inline-option select {
      background: #1a1a1a;
      border-color: #444;
      color: #f0f0f0;
    }
  }

  .threshold-option,
  .time-remaining-option {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .threshold-chips,
  .time-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-left: 20px;
  }

  .threshold-chips.disabled,
  .time-chips.disabled {
    opacity: 0.5;
  }

  .threshold-chip,
  .time-chip {
    padding: 3px 8px;
    border: 1px solid #ddd;
    border-radius: 12px;
    background: #fff;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.15s ease;
    color: #666;
  }

  .threshold-chip:hover:not(:disabled),
  .time-chip:hover:not(:disabled) {
    background: #7c3aed;
    border-color: #7c3aed;
    color: #fff;
  }

  .threshold-chip.selected,
  .time-chip.selected {
    background: #7c3aed;
    border-color: #7c3aed;
    color: #fff;
  }

  .threshold-chip:disabled,
  .time-chip:disabled {
    cursor: not-allowed;
  }

  @media (prefers-color-scheme: dark) {
    .threshold-chip,
    .time-chip {
      background: #1a1a1a;
      border-color: #444;
      color: #aaa;
    }

    .threshold-chip:hover:not(:disabled),
    .time-chip:hover:not(:disabled) {
      background: #9f7aea;
      border-color: #9f7aea;
      color: #fff;
    }

    .threshold-chip.selected,
    .time-chip.selected {
      background: #7c3aed;
      border-color: #7c3aed;
      color: #fff;
    }
  }
</style>
