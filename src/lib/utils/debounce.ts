/**
 * Creates a debounced version of a function that delays execution
 * until after the specified wait time has elapsed since the last call.
 */
export function debounce<T extends (...args: Parameters<T>) => void>(
	fn: T,
	wait: number,
): (...args: Parameters<T>) => void {
	let timeoutId: ReturnType<typeof setTimeout> | null = null;

	return (...args: Parameters<T>) => {
		if (timeoutId !== null) {
			clearTimeout(timeoutId);
		}
		timeoutId = setTimeout(() => {
			fn(...args);
			timeoutId = null;
		}, wait);
	};
}
