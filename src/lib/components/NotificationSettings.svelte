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

  function updateThresholds(usageType: UsageType, value: string) {
    const thresholds = value
      .split(",")
      .map((s) => Number.parseInt(s.trim(), 10))
      .filter((n) => !Number.isNaN(n) && n >= 0 && n <= 100)
      .sort((a, b) => a - b);
    updateRule(usageType, { thresholds });
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
                <input
                  type="text"
                  value={rule.thresholds.join(", ")}
                  placeholder="80, 90"
                  disabled={!rule.threshold_enabled}
                  onchange={(e) =>
                    updateThresholds(usageType, e.currentTarget.value)}
                />
              </label>
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

  .inline-option select,
  .inline-option input[type="text"] {
    padding: 4px 6px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.8rem;
    background: #fff;
  }

  .inline-option select:disabled,
  .inline-option input[type="text"]:disabled {
    opacity: 0.5;
  }

  @media (prefers-color-scheme: dark) {
    .inline-option select,
    .inline-option input[type="text"] {
      background: #1a1a1a;
      border-color: #444;
      color: #f0f0f0;
    }
  }

  .inline-option input[type="text"] {
    width: 80px;
  }
</style>
