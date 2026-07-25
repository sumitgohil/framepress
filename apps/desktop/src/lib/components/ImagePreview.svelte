<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Image as ImageIcon } from "lucide-svelte";

  type Props = {
    /** Ordered paths to try. The optimized file normally comes first. */
    paths: Array<string | null | undefined>;
    alt?: string;
    size?: number;
  };

  let { paths, alt = "", size = 18 }: Props = $props();
  let source_index = $state(0);
  let paths_key = $derived(
    paths.filter((path): path is string => Boolean(path)).join("\u0000"),
  );
  let available_paths = $derived(
    paths.filter((path): path is string => Boolean(path)),
  );
  let current_path = $derived(available_paths[source_index] ?? null);
  let source = $derived(current_path ? convertFileSrc(current_path) : null);

  // Reset the fallback sequence when this component is reused for another row.
  $effect(() => {
    paths_key;
    source_index = 0;
  });

  function try_next_source() {
    source_index += 1;
  }
</script>

{#if source}
  <img
    src={source}
    {alt}
    class="h-full w-full object-cover"
    onerror={try_next_source}
  />
{:else}
  <ImageIcon {size} />
{/if}
