<script lang="ts">
  import Fader from './Fader.svelte';
  import { live, type LiveState, type Track } from './live.svelte.ts';
  import Meter from './Meter.svelte';
  import TrackButtons from './TrackButtons.svelte';
  import { DEFAULT_VOLUME, textOn, visibleTracks } from './util.ts';

  let { data }: { data: LiveState } = $props();

  const all = $derived([...visibleTracks(data), ...data.returns, data.master]);

  function setVolume(t: Track, v: number) {
    live.cmdThrottled(`${t.ref}.volume`, 'set_track', { track: t.ref, prop: 'volume', value: v });
  }
</script>

<div class="list">
  {#each all as t (t.ref)}
    <div class="item">
      <div class="top">
        <span class="name" style:background={t.color} style:color={textOn(t.color)}>{t.group ? '↳ ' : ''}{t.name}</span>
        <span class="db">{t.volumeStr}</span>
      </div>
      <div class="meter"><Meter value={live.meter(t.ref)} vertical={false} /></div>
      <Fader vertical={false} value={t.volume} defaultValue={DEFAULT_VOLUME} onchange={(v) => setVolume(t, v)} />
      <TrackButtons track={t} />
    </div>
  {/each}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    padding-left: max(12px, env(safe-area-inset-left));
    padding-right: max(12px, env(safe-area-inset-right));
    padding-bottom: max(16px, env(safe-area-inset-bottom));
    overflow-y: auto;
    height: 100%;
  }
  .item {
    background: var(--panel);
    border-radius: 8px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .item :global(.buttons) {
    justify-content: flex-start;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .name {
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .db {
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .meter {
    height: 8px;
  }
</style>
