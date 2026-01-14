<script lang="ts">
  import type { Toast } from "$lib/composables";

  interface Props {
    toasts: Toast[];
    onDismiss: (id: number) => void;
  }

  let { toasts, onDismiss }: Props = $props();

  const alertClasses: Record<Toast["type"], string> = {
    success: "alert-success",
    error: "alert-error",
    warning: "alert-warning",
    info: "alert-info",
  };
</script>

{#if toasts.length > 0}
  <div class="toast toast-end">
    {#each toasts as t (t.id)}
      <div class="alert {alertClasses[t.type]} gap-2">
        <span>{t.message}</span>
        <button class="btn btn-ghost btn-xs btn-circle" onclick={() => onDismiss(t.id)}>✕</button>
      </div>
    {/each}
  </div>
{/if}
