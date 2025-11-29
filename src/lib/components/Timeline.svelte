<script>
  export let compact = false;

  import {
    currentPosition,
    totalDuration,
    progress,
    formatTime,
    seekTo
  } from '../stores/player.js';
  import { bandStatus, isHost, bandSeek } from '../stores/band.js';

  let isDragging = false;
  let progressBar;

  function handleSeek(e) {
    if (!progressBar || $totalDuration === 0) return;

    const rect = progressBar.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const percent = Math.max(0, Math.min(1, x / rect.width));
    const position = percent * $totalDuration;

    // Use band seek if in band mode and host, otherwise normal seek
    if ($bandStatus === 'connected' && $isHost) {
      bandSeek(position);
    } else {
      seekTo(position);
    }
  }

  function handleMouseDown(e) {
    isDragging = true;
    handleSeek(e);
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  function handleMouseMove(e) {
    if (isDragging) {
      handleSeek(e);
    }
  }

  function handleMouseUp() {
    isDragging = false;
    window.removeEventListener('mousemove', handleMouseMove);
    window.removeEventListener('mouseup', handleMouseUp);
  }
</script>

<div class="mt-2 {compact ? 'py-1' : ''}">
  <div class="flex items-center gap-2">
    <span class="text-[11px] text-white/70 min-w-[40px]">
      {formatTime($currentPosition)}
    </span>

    <div
      bind:this={progressBar}
      class="group flex-1 h-1 bg-white/20 rounded-full relative cursor-pointer"
      role="slider"
      tabindex="0"
      aria-valuemin="0"
      aria-valuemax={$totalDuration}
      aria-valuenow={$currentPosition}
      aria-label="Seek playback position"
      onmousedown={handleMouseDown}
    >
      <div
        class="h-full bg-white rounded-full relative group-hover:bg-[#1db954] pointer-events-none"
        style="width: {$progress}%; transition: {isDragging ? 'none' : 'width 0.1s'};"
      >
        <!-- Playhead - only visible on hover or dragging -->
        <div
          class="absolute right-0 top-1/2 -translate-y-1/2 w-3 h-3 bg-white rounded-full shadow-lg transition-opacity {isDragging ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}"
        ></div>
      </div>
    </div>

    <span class="text-[11px] text-white/70 min-w-[40px] text-right">
      {formatTime($totalDuration)}
    </span>
  </div>
</div>
