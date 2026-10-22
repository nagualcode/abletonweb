<script lang="ts">
  import { live, type Track } from './live.svelte.ts';
  import { monitorLabel } from './util.ts';

  // Mute (track activator), solo, arm and monitoring for one track.
  let { track, monitor = true }: { track: Track; monitor?: boolean } = $props();

  const label = $derived(track.ref[0] === 'r' ? 'ABCDEFGHIJKL'[Number(track.ref.slice(1))] : String(Number(track.ref.slice(1)) + 1));
  const MONITORS = ['in', 'auto', 'off'] as const;

  function set(prop: string, value: unknown, extra: Record<string, unknown> = {}) {
    live.cmd('set_track', { track: track.ref, prop, value, ...extra });
  }

  function nextMonitor() {
    const i = MONITORS.indexOf(track.monitor ?? 'auto');
    set('monitor', MONITORS[(i + 1) % MONITORS.length]);
  }
</script>

<div class="buttons">
  {#if track.mute !== null}
    <button class="act" class:on={!track.mute} title="Mute" onclick={() => set('mute', !track.mute)}>{label}</button>
  {/if}
  {#if track.solo !== null}
    <button class="solo" class:on={track.solo} title="Solo" onclick={(e) => set('solo', !track.solo, { exclusive: !(e.metaKey || e.ctrlKey) })}>S</button>
  {/if}
  {#if track.canArm}
    <button class="arm" class:on={track.arm} title="Arm" onclick={(e) => set('arm', !track.arm, { exclusive: !(e.metaKey || e.ctrlKey) })}>●</button>
    {#if monitor}
      <button class="mon" class:on={track.monitor !== 'auto'} title="Monitor" onclick={nextMonitor}>{monitorLabel(track.monitor)}</button>
    {/if}
  {/if}
</div>

<style>
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    justify-content: center;
  }
  button {
    min-width: 30px;
    padding: 5px 4px;
    font-size: 12px;
  }
  .act.on {
    background: var(--accent);
    color: var(--bg);
  }
  .solo.on {
    background: var(--solo);
    color: var(--bg);
  }
  .arm.on {
    background: var(--rec);
    color: #fff;
  }
  .mon {
    min-width: 40px;
  }
  .mon.on {
    background: var(--monitor);
    color: var(--bg);
  }
</style>
