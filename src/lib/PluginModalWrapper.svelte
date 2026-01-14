<script>
  import { onMount, onDestroy } from 'svelte';

  // Wrapper component for dynamically rendering plugin modals
  let { component, props = {} } = $props();

  let container;
  let cleanupFn = null;

  onMount(() => {
    if (!container || !component) return;

    console.log('[PluginModalWrapper] Mounting component:', component.name);
    console.log('[PluginModalWrapper] Props received:', props);

    try {
      const propsToPass = props && typeof props === 'object' ? props : {};
      console.log('[PluginModalWrapper] Calling component function');

      // Call component as a plain function: component(target, props)
      cleanupFn = component(container, propsToPass);

      console.log('[PluginModalWrapper] Component mounted successfully');
    } catch (e) {
      console.error('[PluginModalWrapper] Failed to mount component:', e);
      console.error('[PluginModalWrapper] Stack:', e.stack);
    }
  });

  onDestroy(() => {
    if (cleanupFn && typeof cleanupFn === 'function') {
      try {
        console.log('[PluginModalWrapper] Calling cleanup function');
        cleanupFn();
        cleanupFn = null;
      } catch (e) {
        console.error('[PluginModalWrapper] Error during cleanup:', e);
      }
    }
  });
</script>

<div bind:this={container}></div>
