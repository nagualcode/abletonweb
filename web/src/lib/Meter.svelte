<script lang="ts">
  let { value, vertical = true }: { value: [number, number]; vertical?: boolean } = $props();

  function level(v: number): string {
    return v > 0.94 ? 'clip' : v > 0.8 ? 'hot' : '';
  }
</script>

<div class="meter" class:vertical>
  {#each value as v, i (i)}
    <div class="ch">
      <div class="fill {level(v)}" style:--v={Math.min(1, v)}></div>
    </div>
  {/each}
</div>

<style>
  .meter {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    height: 100%;
  }
  .meter.vertical {
    flex-direction: row;
  }
  .ch {
    flex: 1;
    background: var(--meter-bg);
    border-radius: 2px;
    overflow: hidden;
    position: relative;
  }
  .fill {
    position: absolute;
    inset: 0;
    background: var(--meter);
    transform-origin: left center;
    transform: scaleX(var(--v));
    transition: transform 100ms linear;
  }
  .vertical .fill {
    transform-origin: center bottom;
    transform: scaleY(var(--v));
  }
  .fill.hot {
    background: var(--meter-hot);
  }
  .fill.clip {
    background: var(--rec);
  }
</style>
