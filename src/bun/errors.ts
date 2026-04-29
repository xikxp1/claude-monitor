export class AppError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AppError";
  }
}

export const INVALID_TOKEN = "Authentication expired. Refresh your provider login and try again.";
export const RATE_LIMITED = "Rate limited. Please wait a moment and try again.";
export const NETWORK_ERROR = "Network error. Check your internet connection.";

export function normalizeError(error: unknown): string {
  if (error instanceof AppError) {
    return error.message;
  }

  if (error instanceof TypeError && error.message.toLowerCase().includes("fetch")) {
    return NETWORK_ERROR;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export function missingConfig(name: string): AppError {
  return new AppError(`Missing configuration: ${name}`);
}
