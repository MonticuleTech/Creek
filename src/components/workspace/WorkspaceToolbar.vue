<template>
  <div class="toolbar">
    <div class="toolbar-group">
      <button 
        @click="$emit('start-recording')" 
        :disabled="isRecording || isPaused"
        class="toolbar-btn"
        :class="{ 'recording': isRecording && !isPaused }"
        title="Start Recording"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="12" r="10"/>
        </svg>
      </button>
      <button 
        @click="$emit('toggle-pause')" 
        :disabled="!isRecording"
        class="toolbar-btn"
        :class="{ 'paused': isPaused }"
        title="Pause/Resume Recording"
      >
        <svg v-if="!isPaused" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="4" width="4" height="16" rx="1"/>
          <rect x="14" y="4" width="4" height="16" rx="1"/>
        </svg>
        <svg v-else width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <polygon points="5 3 19 12 5 21"/>
        </svg>
      </button>
      <button 
        @click="$emit('stop-recording')" 
        :disabled="!isRecording"
        class="toolbar-btn"
        title="Stop Recording"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="6" width="12" height="12" rx="2"/>
        </svg>
      </button>
      <div v-if="isRecording && !isPaused" class="recording-dot"></div>
      <div v-if="isPaused" class="paused-indicator">Paused</div>
    </div>
    
    <div class="toolbar-divider"></div>
    
    <div class="toolbar-group">
      <button @click="$emit('save-document')" class="toolbar-btn" title="Save Document">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
          <polyline points="17 21 17 13 7 13 7 21"/>
          <polyline points="7 3 7 8 15 8"/>
        </svg>
      </button>
      <button @click="$emit('reset-document')" class="toolbar-btn" title="Reset Document">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="1 4 1 10 7 10"/>
          <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/>
        </svg>
      </button>
      
      <div class="mic-bar" :class="{ active: isRecording && !isPaused }" title="Microphone status">
        <span class="mic-bar-label">Mic</span>
        <span class="mic-bar-track">
          <span class="mic-bar-fill" :style="{ width: micVolume + '%' }"></span>
        </span>
      </div>
    </div>

    <div class="toolbar-spacer"></div>

    <div class="toolbar-group">
      <button @click="$emit('new-document')" class="toolbar-btn new-btn" title="New Document">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="12" y1="11" x2="12" y2="17"/>
          <line x1="9" y1="14" x2="15" y2="14"/>
        </svg>
      </button>
    </div>
  </div>
</template>

<script>
export default {
  name: 'WorkspaceToolbar',
  props: {
    isRecording: {
      type: Boolean,
      default: false
    },
    isPaused: {
      type: Boolean,
      default: false
    },
    micVolume: {
      type: Number,
      default: 0
    }
  },
  emits: [
    'start-recording',
    'stop-recording',
    'toggle-pause',
    'save-document',
    'reset-document',
    'new-document'
  ]
};
</script>

<style scoped>
/* Toolbar */
.toolbar {
  display: flex;
  align-items: center;
  height: 44px; /* var(--header-height) */
  padding: 0 12px;
  background: #eef2e4;
  border-bottom: 1px solid #c1c989;
  gap: 8px;
  flex-shrink: 0;
  box-sizing: border-box;
  user-select: none;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-btn {
  background: transparent;
  border: none;
  color: #9fb296;
  cursor: pointer;
  width: 32px;
  height: 32px;
  padding: 6px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  box-sizing: border-box;
}

.mic-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 8px;
  border: 1px solid #c1c989;
  border-radius: 16px;
  background: #f4f7eb;
  color: #7d9569;
  font-size: 12px;
  letter-spacing: 0.2px;
  box-sizing: border-box;
  min-width: 86px;
}

.mic-bar-label {
  font-size: 12px;
  color: #7d9569;
  line-height: 1;
}

.mic-bar-track {
  position: relative;
  width: 56px;
  height: 6px;
  border-radius: 999px;
  background: #d9e2c0;
  overflow: hidden;
}

.mic-bar-fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  width: 0%;
  border-radius: 999px;
  background: #b3c28a;
  transition: none;
}

.mic-bar.active .mic-bar-fill {
  background: #7d9569;
}

.toolbar-btn:hover:not(:disabled) {
  background: #e0e7a0;
  color: #5e6c53;
}

.toolbar-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.toolbar-btn.recording {
  color: #c17a7a;
  animation: pulse 1s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.recording-dot {
  width: 8px;
  height: 8px;
  background: #c17a7a;
  border-radius: 50%;
  animation: pulse 1s infinite;
  margin-left: 4px;
}

.toolbar-divider {
  width: 1px;
  height: 20px;
  background: #c1c989;
  margin: 0 4px;
}

.toolbar-spacer {
  flex: 1;
}

.toolbar-btn.new-btn {
  color: #7d9569;
  border: 1px solid #c1c989;
  border-radius: 6px;
  padding: 6px;
}

.toolbar-btn.new-btn:hover:not(:disabled) {
  background: #7d9569;
  color: #ffffff;
  border-color: #7d9569;
}

.paused-indicator {
  font-size: 12px;
  color: #d4a373;
  font-weight: 500;
  margin-left: 4px;
}

.toolbar-btn.paused {
  color: #d4a373;
}
</style>
