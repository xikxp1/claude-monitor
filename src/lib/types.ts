export interface UsagePeriod {
  utilization: number;
  resets_at: string;
}

export interface UsageData {
  five_hour: UsagePeriod | null;
  seven_day: UsagePeriod | null;
  seven_day_sonnet: UsagePeriod | null;
  seven_day_opus: UsagePeriod | null;
}

export interface Settings {
  organization_id: string | null;
  session_token: string | null;
  refresh_interval_minutes: number;
}
