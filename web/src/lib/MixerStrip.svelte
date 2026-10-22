<script lang="ts">
  import Fader from './Fader.svelte';
  import { live, type Track } from './live.svelte.ts';
  import Meter from './Meter.svelte';
  import TrackButtons from './TrackButtons.svelte';
  import { DEFAULT_VOLUME } from './util.ts';

  let { track }: { track: Track } = $props();

  function set(prop: string, value: number, extra: Record<string, unknown> = {}) {
    live.cmdThrottled(`${track.ref}.${prop}.${extra.index ?? ''}`, 'set_track',
      { track: track.ref, prop, value, ...extra });
  }
</script>

<div class="strip">
  {#each track.sends as send, i (i)}
    <div class="send" title="Send {'ABCDEFGHIJKL'[i]}: {send.str}">
      <span>{'ABCDEFGHIJKL'[i]}</span>
      <Fader vertical={false} value={send.value} defaultValue={0} color="var(--send)" onchange={(v) => set('send', v, { index: i })} />
    </div>
  {/each}
  <div class="pan" title="Pan {track.panStr}">
    <Fader vertical={false} bipolar value={track.pan} min={-1} max={1} defaultValue={0} color="var(--muted)" onchange={(v) => set('pan', v)} />
    <span class="small">{track.panStr}</span>
  </div>
  <div class="level">
    <div class="meter"><Meter value={live.meter(track.ref)} /></div>
    <Fader value={track.volume} defaultValue={DEFAULT_VOLUME} onchange={(v) => set('volume', v)} />
  </div>
  <span class="small db">{track.volumeStr}</span>
  <TrackButtons {track} />
</div>

<style>
  .strip {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    padding: 8px 6px;
    height: 100%;
  }
  .send,
  .pan {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .send span {
    font-size: 10px;
    color: var(--muted);
    width: 10px;
  }
  .pan .small {
    min-width: 26px;
    text-align: right;
  }
  .level {
    flex: 1;
    min-height: 100px;
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .meter {
    width: 12px;
  }
  .small {
    font-size: 11px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .db {
    text-align: center;
  }
</style>
