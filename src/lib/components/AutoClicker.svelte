<script>
  import Icon from "@iconify/svelte";
  import { fly, fade } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { t } from "svelte-i18n";

  // Auto clicker state
  let isEnabled = false;
  let key = "f";
  let delayMs = 100;
  let hotkey = "F8";
  let showSettings = false;
  let isRecordingHotkey = false;
  
  // Secondary key state
  let key2 = "";
  let delay2Ms = 200;
  let enabled2 = false;
  
  // Window detection status
  let gameWindowFound = false;
  let cloudMode = false;

  // Cleanup
  let unlistenToggle = null;
  let unlistenKeyCapture = null;
  let refreshInterval = null;

  async function loadConfig() {
    try {
      const config = await invoke('auto_clicker_get_config');
      key = config.key;
      delayMs = config.delay_ms;
      isEnabled = config.enabled;
      hotkey = config.hotkey;
      gameWindowFound = config.game_window_found;
      cloudMode = config.cloud_mode;
      // Secondary key
      key2 = config.key2 || "";
      delay2Ms = config.delay2_ms || 200;
      enabled2 = config.enabled2 || false;
    } catch (e) {
      console.error('Failed to load auto clicker config:', e);
    }
  }

  onMount(async () => {
    // Load current config
    await loadConfig();

    // Refresh window status periodically when settings panel is open
    refreshInterval = setInterval(async () => {
      if (showSettings) {
        await loadConfig();
      }
    }, 2000);

    // Listen for hotkey toggle events from backend
    unlistenToggle = await listen('auto-clicker-toggled', (event) => {
      isEnabled = event.payload;
    });

    // Listen for key capture (for hotkey recording)
    unlistenKeyCapture = await listen('key-captured', (event) => {
      if (isRecordingHotkey) {
        const keyName = event.payload;
        if (keyName === 'Escape') {
          stopRecordingHotkey();
        } else {
          applyHotkey(keyName);
        }
      }
    });
  });

  onDestroy(() => {
    if (unlistenToggle) unlistenToggle();
    if (unlistenKeyCapture) unlistenKeyCapture();
    if (refreshInterval) clearInterval(refreshInterval);
  });

  async function toggle() {
    try {
      isEnabled = await invoke('auto_clicker_toggle');
    } catch (e) {
      console.error('Failed to toggle auto clicker:', e);
    }
  }

  async function updateKey(newKey) {
    const trimmed = newKey.toLowerCase().trim();
    if (trimmed.length === 0) return;
    
    try {
      await invoke('auto_clicker_set_key', { key: trimmed });
      key = trimmed;
    } catch (e) {
      console.error('Failed to set auto clicker key:', e);
    }
  }

  async function updateDelay(newDelay) {
    const parsed = parseInt(newDelay);
    if (isNaN(parsed) || parsed < 10) return;
    
    try {
      await invoke('auto_clicker_set_delay', { delayMs: parsed });
      delayMs = parsed;
    } catch (e) {
      console.error('Failed to set auto clicker delay:', e);
    }
  }

  async function updateKey2(newKey) {
    const trimmed = newKey.toLowerCase().trim();
    
    try {
      await invoke('auto_clicker_set_key2', { key: trimmed });
      key2 = trimmed;
      // Auto-enable key2 if it has a value
      if (trimmed.length > 0 && !enabled2) {
        await toggleKey2Enabled(true);
      } else if (trimmed.length === 0 && enabled2) {
        await toggleKey2Enabled(false);
      }
    } catch (e) {
      console.error('Failed to set auto clicker key2:', e);
    }
  }

  async function updateDelay2(newDelay) {
    const parsed = parseInt(newDelay);
    if (isNaN(parsed) || parsed < 10) return;
    
    try {
      await invoke('auto_clicker_set_delay2', { delayMs: parsed });
      delay2Ms = parsed;
    } catch (e) {
      console.error('Failed to set auto clicker delay2:', e);
    }
  }

  async function toggleKey2Enabled(forceState = null) {
    const newState = forceState !== null ? forceState : !enabled2;
    
    try {
      await invoke('auto_clicker_set_key2_enabled', { enabled: newState });
      enabled2 = newState;
    } catch (e) {
      console.error('Failed to toggle key2 enabled:', e);
    }
  }

  async function startRecordingHotkey() {
    await invoke('cmd_set_keybindings_enabled', { enabled: false });
    isRecordingHotkey = true;
    await invoke('cmd_unfocus_window');
  }

  async function stopRecordingHotkey() {
    isRecordingHotkey = false;
    await invoke('cmd_set_keybindings_enabled', { enabled: true });
  }

  async function applyHotkey(keyName) {
    try {
      await invoke('auto_clicker_set_hotkey', { hotkey: keyName });
      hotkey = keyName;
    } catch (e) {
      console.error('Failed to set auto clicker hotkey:', e);
    }
    stopRecordingHotkey();
  }

  function handleKeyInput(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateKey(e.target.value);
    }
  }

  function handleDelayInput(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateDelay(e.target.value);
    }
  }

  function handleKey2Input(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateKey2(e.target.value);
    }
  }

  function handleDelay2Input(e) {
    if (e.key === 'Enter') {
      e.target.blur();
      updateDelay2(e.target.value);
    }
  }
</script>

<!-- Compact Auto Clicker Control (for player bar) -->
<div class="relative">
  <!-- Toggle Button -->
  <button
    class="flex items-center gap-1.5 px-2 py-1 rounded-md transition-all text-xs font-medium {isEnabled 
      ? 'bg-orange-500/20 text-orange-400 border border-orange-500/30' 
      : 'text-white/50 hover:text-white hover:bg-white/5'}"
    onclick={() => { showSettings = !showSettings; if (showSettings) loadConfig(); }}
    title={$t("autoClicker.title")}
  >
    <Icon icon={isEnabled ? "mdi:cursor-default-click" : "mdi:cursor-default-click-outline"} class="w-3.5 h-3.5 {isEnabled ? 'animate-pulse' : ''}" />
    <span class="uppercase">{key}{enabled2 && key2 ? `+${key2}` : ''}</span>
    {#if isEnabled}
      <span class="text-[10px] opacity-70">{delayMs}ms{enabled2 && key2 ? `/${delay2Ms}ms` : ''}</span>
    {/if}
  </button>

  <!-- Settings Dropdown -->
  {#if showSettings}
    <button class="fixed inset-0 z-40" onclick={() => showSettings = false}></button>
    <div
      class="absolute bottom-full right-0 mb-2 bg-[#282828] rounded-lg shadow-xl border border-white/10 overflow-hidden z-50 w-64"
      in:fly={{ y: 10, duration: 150 }}
      out:fade={{ duration: 100 }}
    >
      <div class="p-3 space-y-3">
        <!-- Header with Toggle -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Icon icon="mdi:cursor-default-click" class="w-4 h-4 text-orange-400" />
            <span class="font-medium text-sm">{$t("autoClicker.title")}</span>
          </div>
          <button
            class="relative w-10 h-5 rounded-full transition-colors duration-200 {isEnabled
              ? 'bg-orange-500'
              : 'bg-white/20'}"
            onclick={toggle}
          >
            <div
              class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 {isEnabled
                ? 'translate-x-5'
                : 'translate-x-0.5'}"
            ></div>
          </button>
        </div>

        <!-- Key Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.key")}</label>
          <input
            type="text"
            value={key}
            maxlength="1"
            class="w-full px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center uppercase focus:outline-none focus:ring-1 focus:ring-orange-500"
            onkeydown={handleKeyInput}
            onblur={(e) => updateKey(e.target.value)}
          />
        </div>

        <!-- Delay Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.delay")}</label>
          <div class="flex items-center gap-2">
            <input
              type="number"
              value={delayMs}
              min="10"
              max="5000"
              step="10"
              class="flex-1 px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center focus:outline-none focus:ring-1 focus:ring-orange-500"
              onkeydown={handleDelayInput}
              onblur={(e) => updateDelay(e.target.value)}
            />
            <span class="text-xs text-white/40">ms</span>
          </div>
        </div>

        <!-- Divider -->
        <div class="border-t border-white/10 my-1"></div>

        <!-- Secondary Key Section -->
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs text-white/50">{$t("autoClicker.secondaryKey")}</span>
          <button
            class="relative w-8 h-4 rounded-full transition-colors duration-200 {enabled2 && key2
              ? 'bg-orange-500'
              : 'bg-white/20'}"
            onclick={() => toggleKey2Enabled()}
            disabled={!key2}
          >
            <div
              class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow transition-transform duration-200 {enabled2 && key2
                ? 'translate-x-4'
                : 'translate-x-0.5'}"
            ></div>
          </button>
        </div>

        <!-- Secondary Key Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.key2")}</label>
          <input
            type="text"
            value={key2}
            maxlength="1"
            placeholder="-"
            class="w-full px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center uppercase focus:outline-none focus:ring-1 focus:ring-orange-500 placeholder:text-white/20"
            onkeydown={handleKey2Input}
            onblur={(e) => updateKey2(e.target.value)}
          />
        </div>

        <!-- Secondary Delay Input -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.delay2")}</label>
          <div class="flex items-center gap-2">
            <input
              type="number"
              value={delay2Ms}
              min="10"
              max="5000"
              step="10"
              class="flex-1 px-2 py-1.5 bg-white/5 border border-white/10 rounded text-sm text-white text-center focus:outline-none focus:ring-1 focus:ring-orange-500"
              onkeydown={handleDelay2Input}
              onblur={(e) => updateDelay2(e.target.value)}
            />
            <span class="text-xs text-white/40">ms</span>
          </div>
        </div>

        <!-- Divider -->
        <div class="border-t border-white/10 my-1"></div>

        <!-- Hotkey -->
        <div>
          <label class="text-xs text-white/50 mb-1 block">{$t("autoClicker.hotkey")}</label>
          <button
            class="w-full px-2 py-1.5 rounded text-sm font-mono text-center transition-all {isRecordingHotkey 
              ? 'bg-orange-500 text-black animate-pulse' 
              : 'bg-white/10 hover:bg-white/20 text-white'}"
            onclick={startRecordingHotkey}
          >
            {isRecordingHotkey ? '...' : hotkey}
          </button>
        </div>

        <!-- Status Indicator -->
        <div class="flex items-center gap-2 px-2 py-1.5 rounded bg-white/5">
          {#if cloudMode}
            <Icon icon="mdi:cloud" class="w-3.5 h-3.5 text-blue-400" />
            <span class="text-xs text-blue-400">Cloud Mode (needs focus)</span>
          {:else if gameWindowFound}
            <Icon icon="mdi:check-circle" class="w-3.5 h-3.5 text-green-400" />
            <span class="text-xs text-green-400">Background mode ready</span>
          {:else}
            <Icon icon="mdi:alert-circle" class="w-3.5 h-3.5 text-yellow-400" />
            <span class="text-xs text-yellow-400">Game window not found</span>
          {/if}
        </div>

        <!-- Info -->
        <p class="text-xs text-white/40">
          {$t("autoClicker.description")}
        </p>
      </div>
    </div>
  {/if}
</div>






