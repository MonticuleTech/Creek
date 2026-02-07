<script setup>
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import LiveCanvas from '../components/LiveCanvas.vue';
import { onDocumentUpdate, onLlmStream } from '../api/events';
import { invoke } from '@tauri-apps/api/core';

const docContent = ref('');
const isRecording = ref(false);
const streamContent = ref('');
const isThinking = ref(false);
const streamAction = ref('thinking');
const streamScrollRef = ref(null);

let unlistenDoc;
let unlistenStream;

const toggleRecording = async () => {
    if (isRecording.value) {
        await invoke('stop_recording');
        isRecording.value = false;
    } else {
        await invoke('start_recording');
        isRecording.value = true;
    }
};

const reset = async () => {
    await invoke('reset_document');
};

const onContentChange = async (newContent) => {
    // When user types, update backend SSOT
    await invoke('update_document', { content: newContent });
    // Optimistic update
    docContent.value = newContent;
};

onMounted(async () => {
  console.log("HomeView mounted");

  unlistenDoc = await onDocumentUpdate((event) => {
    console.log('Received document update:', event);
    const appEvent = event.payload;
    if (appEvent && appEvent.type === 'DocumentUpdate' && appEvent.payload) {
        docContent.value = appEvent.payload.content;
    }
  });

  unlistenStream = await onLlmStream(async (event) => {
      const payload = event.payload;
      if (payload.phase === 'start') {
          isThinking.value = true;
          streamContent.value = '';
          streamAction.value = 'thinking';
      } else if (payload.phase === 'chunk') {
          streamContent.value += payload.content;
          streamAction.value = payload.action_type;
          // Auto-scroll
          await nextTick();
          if (streamScrollRef.value) {
              streamScrollRef.value.scrollTop = streamScrollRef.value.scrollHeight;
          }
      } else if (payload.phase === 'end') {
          // Keep showing for a moment or until next start?
          // Let's keep it visible but maybe indicate done.
          isThinking.value = false;
      }
  });
});

onUnmounted(() => {
  if (unlistenDoc) unlistenDoc();
  if (unlistenStream) unlistenStream();
  // Ensure recording stops if component unmounts
  if (isRecording.value) invoke('stop_recording');
});
</script>

<template>
  <div class="canvas-layout d-flex flex-column">
    <!-- Toolbar -->
    <div class="toolbar px-2 py-1 border-bottom d-flex align-items-center justify-content-between bg-white">
        <div class="d-flex align-items-center gap-2">
            <button 
              class="btn btn-sm rounded-pill d-flex align-items-center gap-2 px-3"
              :class="isRecording ? 'btn-danger' : 'btn-primary'"
              @click="toggleRecording"
            >
                <span v-if="isRecording" class="spinner-grow spinner-grow-sm" role="status" aria-hidden="true"></span>
                {{ isRecording ? 'Stop Recording' : 'Start Recording' }}
            </button>
            
            <button class="btn btn-sm btn-outline-secondary rounded-pill" @click="reset">
                Reset
            </button>
        </div>
        <div class="text-muted small">
            {{ isRecording ? 'Listening...' : 'Ready' }}
        </div>
    </div>

    <div class="flex-grow-1 overflow-hidden position-relative">
        <LiveCanvas 
          :content="docContent" 
          @update:content="onContentChange"
        />
        
        <!-- Stream/Thinking Overlay (Hidden for now as we stream directly to doc) -->
        <!-- <div v-if="streamContent" class="stream-overlay border-top bg-light p-3" :class="{ 'thinking': isThinking }"> ... </div> -->
    </div>
  </div>
</template>

<style scoped>
.canvas-layout {
    height: 100vh;
    width: 100%;
    overflow: hidden;
    background-color: #fff;
}
.toolbar {
    height: 44px;
    z-index: 10;
}
.stream-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    max-height: 40%;
    box-shadow: 0 -2px 10px rgba(0,0,0,0.05);
    z-index: 20;
    transition: all 0.3s ease;
    opacity: 0.95;
}
.stream-overlay.thinking {
    border-color: #86b7fe !important; /* highlight border when active */
}
.d-flex {
  display: flex;
}
.flex-column {
  flex-direction: column;
}
.position-relative {
    position: relative;
}
</style>
