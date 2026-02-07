<template>
  <Teleport to="body">
    <Transition name="toast-fade">
      <div v-if="visible" class="toast-container" :class="typeClass">
        <div class="toast-content">
          <svg v-if="type === 'info'" class="toast-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="16" x2="12" y2="12"/>
            <line x1="12" y1="8" x2="12.01" y2="8"/>
          </svg>
          <svg v-else-if="type === 'warning'" class="toast-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
            <line x1="12" y1="9" x2="12" y2="13"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
          <div class="toast-message">{{ message }}</div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script>
import { ref, watch, onMounted, onUnmounted } from 'vue';
import { listen } from '@tauri-apps/api/event';

export default {
  name: 'Toast',
  setup() {
    const visible = ref(false);
    const message = ref('');
    const type = ref('info'); // 'info' or 'warning'
    const typeClass = ref('toast-info');
    let hideTimer = null;
    let unlisten = null;

    const show = (msg, toastType = 'info', duration = 3000) => {
      message.value = msg;
      type.value = toastType;
      typeClass.value = `toast-${toastType}`;
      visible.value = true;

      // Clear previous timer
      if (hideTimer) {
        clearTimeout(hideTimer);
      }

      // Auto hide after duration
      hideTimer = setTimeout(() => {
        visible.value = false;
      }, duration);
    };

    const hide = () => {
      visible.value = false;
      if (hideTimer) {
        clearTimeout(hideTimer);
      }
    };

    onMounted(async () => {
      // Listen for toast events from backend
      unlisten = await listen('show-toast', (event) => {
        const { message: msg, type: toastType, duration } = event.payload;
        show(msg, toastType || 'info', duration || 3000);
      });
    });

    onUnmounted(() => {
      if (unlisten) unlisten();
      if (hideTimer) clearTimeout(hideTimer);
    });

    return {
      visible,
      message,
      type,
      typeClass,
      show,
      hide
    };
  }
};
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 80px;
  right: 24px;
  z-index: 10000;
  max-width: 400px;
  background: #ffffff;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(94, 108, 83, 0.2);
  padding: 16px 20px;
  border-left: 4px solid;
}

.toast-info {
  border-left-color: #7d9569;
}

.toast-warning {
  border-left-color: #d4a373;
}

.toast-content {
  display: flex;
  align-items: center;
  gap: 12px;
}

.toast-icon {
  flex-shrink: 0;
  color: #7d9569;
}

.toast-warning .toast-icon {
  color: #d4a373;
}

.toast-message {
  font-size: 14px;
  color: #5e6c53;
  line-height: 1.5;
}

.toast-fade-enter-active {
  animation: toast-slide-in 0.3s ease-out;
}

.toast-fade-leave-active {
  animation: toast-slide-out 0.2s ease-in;
}

@keyframes toast-slide-in {
  from {
    opacity: 0;
    transform: translateX(100%);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

@keyframes toast-slide-out {
  from {
    opacity: 1;
    transform: translateX(0);
  }
  to {
    opacity: 0;
    transform: translateX(100%);
  }
}
</style>
